# ironwinrm（Rust + WASM + Web）端到端计划与 TDD 矩阵

本文档聚焦：`crates/ironposh-web`（WASM 包）在 Web 端的可用性与可维护性，尤其是 HostCall/PSRP records 的类型安全与“遇到未知/异常数据不崩”的鲁棒性。

## 目标
- Web 端不会因为某个 PSRP message / record 解析失败而“卡死”（stream 不再推进）。
- JS/TS 能拿到足够上下文定位问题来源（哪条 pipeline、最后一个事件、失败的 record 类型等）。
- HostCall/Record 的跨边界数据结构尽量强类型（避免 `any`），字段命名约定一致（建议 camelCase）。
- 对于不可完全解析/不支持的消息：必须走降级路径（lossy / unsupported），而不是 panic/unwrap。

## 端到端落地步骤（建议顺序）
1. **边界类型梳理**：列出 WASM 暴露给 TS 的所有 event/record/hostcall 类型；确认哪些字段是 snake_case、哪些是 camelCase。
2. **失败可观测性**：
   - JS 侧：对 `stream.next()` 增加超时与“最后事件摘要”错误信息（pipelineCreated/finished/record.kind 等）。
   - Rust 侧：解析失败必须 `warn!(...)`（结构化字段），并继续流转。
3. **Records 的降级策略**：
   - 对 `Write-Information` 等 `MessageData: object` 的场景：允许 message 不是字符串，使用 `to_string`/属性 fallback。
   - 对未知 message_type：映射为 `Unsupported` record（携带可读 preview）。
4. **HostCalls 全覆盖**：按 HostCall 分类（HostInfo/Input/Output/RawUI）逐个实现/对齐 tokio client 行为；每加一个，补一条 E2E。
5. **发布与回归**：每次变更后本地 `wasm-pack build` + WebTerminal Playwright E2E 全跑；保证回归可复现。

---

## TDD 矩阵（Rust 侧）
> 目标：保证解析、降级、以及“不会卡死”这些关键语义在 Rust 层就可验证。

### R1：InformationRecord 解析（严格）
- 输入：`InformationalRecord_Message` 是 `string`
- 期望：`InformationRecord::try_from` 成功，字段 roundtrip
- 类型：单元测试（`crates/ironposh-psrp/src/messages/information_record.rs`）

### R2：InformationRecord 解析（MessageData 非 string）
- 输入：`InformationalRecord_Message` 是非字符串（例如复杂对象/数值）
- 期望：严格解析返回 Err；上层必须能走 lossy（不 panic）
- 类型：新增单元测试（psrp 层）+ 新增单元测试（client-core 的 lossy 路径）

### R3：RunspacePool 接收 InformationRecord（lossy 不终止会话）
- 输入：构造一个“无法严格解析”的 InformationRecord message
- 期望：
  - 产生 `PsrpRecord::Information`（message_data 为 fallback string）或 `Unsupported`
  - 不返回 Err，不中断消息循环
- 类型：单元/集成测试（`crates/ironposh-client-core`）

### R4：未知/不支持 message_type
- 输入：未知 message_type 的 payload
- 期望：映射为 `PsrpRecord::Unsupported`，且 pipeline 仍能 `Finished`
- 类型：集成测试（client-core）

---

## TDD 矩阵（WASM/TS 侧）
> 目标：确保浏览器侧不会 hang、不会吞错误，并且输出/hostcall 行为可回归。

### W1：Records（Warning/Verbose/Debug/Information/Progress）
- 执行：分别运行 `Write-Warning/Write-Verbose/Write-Debug/Write-Information/Write-Progress`
- 期望：
  - `runCommand()` 返回（不 hang）
  - 输出包含 marker
  - 会话仍可继续执行下一条命令
- 类型：Playwright E2E（推荐使用真实 gateway）

### W2：Host UI hostcalls
- 执行：`$Host.UI.Write*` 系列（Warning/Error/Verbose/Debug/Write/WriteLine）
- 期望：浏览器侧 hostcall handler 都能接到对应方法名；不 crash
- 类型：Playwright E2E

### W3：RawUI（Clear-Host）
- 执行：`Clear-Host`
- 期望：触发 `ClearScreen` 或 buffer ops（`SetBufferContents*`），且会话仍可继续
- 类型：Playwright E2E

### W4：错误可定位
- 执行：构造会导致解析失败/卡死风险的命令
- 期望：JS 抛出的错误包含：超时、最后事件摘要（last event summary）、命令片段
- 类型：Playwright E2E（断言错误文本）

---

## 错误信息是否足够（给 JS 定位）
最低要求：
- JS 侧 `runCommand()` 在等待 `stream.next()` 时设置超时；超时错误包含：`lastEventSummary`、是否收到 `PipelineCreated`、是否已 `kill`、以及命令片段。
- Rust 侧对无法解析的 record 走 lossy，并用结构化日志记录：`target`、`error`、`command_id/pipeline_id`（不要记录敏感信息）。
