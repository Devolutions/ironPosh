# WASM/PSRP 稳定性与类型安全：端到端计划 + TDD 矩阵

> 目标：让 `crates/ironposh-web` 在生产环境中遇到 **任何** PSRP/Host 输出时都“不断线、不 panic、可观测、类型安全”。
> 当前已知痛点：某些 PSRP record（例如 `WarningRecord`）会触发 wasm `panic -> RuntimeError: unreachable`，导致会话中断。

---

## 范围与定义

### 范围（本计划覆盖）
- Rust（协议/会话核心）：`crates/ironposh-client-core`、`crates/ironposh-psrp`（必要时也涉及 `crates/ironposh-terminal`）。
- WASM API：`crates/ironposh-web`（wasm-bindgen 导出、事件回调、HostCall 交互）。
- TS/JS（类型）：由 `tsify` 生成或手写的 `.d.ts` / hostcall 对象模型（目标是消灭 `any` 与 `format!` 拼装 payload）。

### 不在范围（但会被依赖/影响）
- 具体 UI（demo/终端渲染组件）的样式问题、`Write-Host` 特殊行为（除非它触发协议崩溃）。

### Done（“能在 prod thrive” 的最低标准）
- **不崩溃**：任何 PSRP message/record 都不会触发 panic（尤其是 wasm `unreachable`）。
- **不丢会话**：遇到未知/暂不支持的 message 类型也能继续执行后续命令。
- **可观测**：未知类型会被结构化记录（Rust `tracing`）且可选择上报给前端。
- **类型安全**：HostCall payload/返回值在 Rust 与 TS 两端都强类型；公共 API 不暴露 `any`。

---

## 端到端计划（聚焦 1/2/3）

### 里程碑 1：止血（Rust/WASM 不再 panic）
**目标**：把所有 “Unhandled message_type/record -> panic” 改为可恢复路径（降级/转发事件）。

1. **复现与枚举**
   - 建立最小脚本集合，分别触发：
     - Warning（`Write-Warning`、`$Host.UI.WriteWarningLine`）
     - Verbose（`Write-Verbose -Verbose`）
     - Debug（`Write-Debug -Debug`）
     - Progress（`Write-Progress`）
     - Information（`Write-Information` / `Write-Host` 相关路径）
   - 记录每个脚本在 wasm 下的行为：是否 panic、是否丢输出、是否中断会话。

2. **替换 panic 为“降级处理”**
   - 在 `crates/ironposh-client-core`（例如 `runspace_pool/pool.rs` 的 message 分发）：
     - 将 `panic!/unreachable!/unwrap` 等致命路径替换为：
       - `SessionEvent::RecordUnsupported { message_type, stream, command_id, raw_summary }`（建议）
       - 或最简单的：把内容降级为 `stdout/stderr` 文本输出（保证不断线）
   - Rust 侧统一使用 `tracing` 结构化日志（不要 log secret）：
     - `warn!(message_type = %..., stream = %..., command_id = ?..., "unsupported PSRP record; downgraded");`

3. **验收**
   - 在 wasm 环境下执行上述最小脚本：不 panic、不掉线，至少能继续执行下一条命令。

---

### 里程碑 2：统一 “Record → Event” 模型（减少 TS 猜测）
**目标**：前端只负责渲染，不参与协议语义推断；所有 record/stream 都通过统一事件输出。

1. **Rust 事件模型（建议形态）**
   - `SessionEvent`（或类似名字）新增/稳定化以下事件（按需精简）：
     - `OutputText { stream: OutputStream, text: String }`
     - `RecordText { level: RecordLevel, stream: OutputStream, text: String, command_id: Option<Uuid> }`
     - `Progress { activity_id: i32, parent_activity_id: i32, activity: String, status: String, percent: Option<i32> }`
     - `Unsupported { message_type: u32, stream: OutputStream, command_id: Option<Uuid>, summary: String }`
   - 关键原则：
     - **永不 panic**：未知类型必须走 `Unsupported`。
     - **可节流**：Progress/频繁事件需要可控频率（避免 UI 卡顿）。

2. **WASM 导出**
   - wasm-bindgen 导出一个事件回调或事件队列 API：
     - `on_session_event((event: JsSessionEvent) => void)`
   - `JsSessionEvent` 必须是稳定 schema（serde + tsify），避免字符串拼装。

3. **TS 绑定与渲染策略**
   - 统一在一个地方把 `JsSessionEvent` 映射到：
     - xterm 输出（stdout/stderr）
     - progress 区域（可选）
     - toast/日志（Unsupported）
   - TS 不应该自行判断 “PSRP WarningRecord 应该如何解析”；应由 Rust 产生 `RecordText/Unsupported`。

4. **验收**
   - 用最小脚本触发 Warning/Progress 等：
     - 前端能收到事件（至少 `RecordText` 或 `Unsupported`），并且会话不中断。

---

### 里程碑 3：HostCall 全面强类型化（Rust structs + tsify + TS union）
**目标**：公共 API 不再出现 `any`，HostCall payload 不再由 `format!`/拼字符串生成。

1. **Rust：把 payload 全部改成结构体**
   - 对每个 HostCall payload / result：
     - 定义 `struct` + 统一宏属性：
       - `#[tsify(into_wasm_abi, from_wasm_abi)]`
       - `#[serde(rename_all = "camelCase")]`
   - 规则：
     - 不允许 `format!` 生成 JSON/字段名/协议数据（除非纯日志字符串）。

2. **TS：可判别联合（discriminated union）**
   - `JsHostCall` 定义为：
     - `{ kind: "WriteLine2", params: ... } | { kind: "Prompt", params: ... } | ...`
   - Handler 定义为：
     - `type TypedHostCallHandler = (call: JsHostCall) => HostCallResultMap[call.kind] | Promise<...>`
   - 结果映射 `HostCallResultMap` 由 `tsify` 输出/或集中维护，避免散落的 module augmentation。

3. **验收**
   - TS 编译期可以：
     - 通过 `call.kind` 自动收窄类型
     - 对每个 hostcall 的返回值类型做静态检查
   - Public API（例如 `connect(...)`）不再接受/返回 `any`。

---

## TDD 矩阵（Rust 侧）

> 目标：用测试把 “不 panic + 事件正确 + 可继续执行” 固化下来。

### A. 单元测试（优先）
| 场景 | crate/位置建议 | 测试类型 | 输入 | 期望 |
|---|---|---|---|---|
| 未知 message_type 不 panic | `crates/ironposh-client-core`（分发/解析模块） | unit | 构造未知类型 record | 返回 `SessionEvent::Unsupported`，且不中断状态机 |
| WarningRecord → RecordText/Unsupported | `crates/ironposh-client-core` | unit | 构造 WarningRecord | 产生可显示文本事件；不 panic |
| Verbose/Debug/Information 同上 | `crates/ironposh-client-core` | unit | 各 record | 同上 |
| ProgressRecord 节流策略 | `crates/ironposh-client-core` | unit | 高频 progress 序列 | 输出事件数量受控（固定窗口/间隔） |

### B. 集成测试（协议/会话层）
| 场景 | crate/位置建议 | 测试类型 | 输入 | 期望 |
|---|---|---|---|---|
| “遇到 Warning 后继续执行下一条命令” | `crates/ironposh-client-core/tests/` | integration | 两条命令：先触发 warning，再输出 marker | 两条都完成；事件包含 marker |
| 混合 stream：stdout + stderr + record | 同上 | integration | 触发多 stream | 前端事件顺序可解释（至少不丢/不乱序到崩） |
| 长输出 + record 插入 | 同上 | integration | 大量输出插入 record | 不 OOM、不死锁、不 panic |

### C. 质量门槛
- `cargo test -p ironposh-client-core`
- `cargo test -p ironposh-psrp`
- `cargo clippy --all-targets --all-features -- -D warnings`

---

## TDD 矩阵（WASM 侧：ironposh-web）

### A. wasm-bindgen-test（Node/Headless）
| 场景 | crate/位置建议 | 测试类型 | 输入 | 期望 |
|---|---|---|---|---|
| JS 端能订阅 session events | `crates/ironposh-web/tests/` 或 `src` 内 `#[cfg(test)]` | wasm test | 注册回调并触发模拟事件 | 回调收到 `JsSessionEvent`（schema 正确） |
| Unsupported 事件 schema 稳定 | 同上 | wasm test | 构造 Unsupported | TS 侧可解析字段（camelCase） |
| Progress 事件节流在 JS 可承受 | 同上 | wasm test | 高频 progress 注入 | JS 收到事件数量受控 |

建议命令（示例）：
- `cd D:\\ironwinrm\\crates\\ironposh-web; wasm-pack test --node`

### B. 端到端（web demo / Playwright）
| 场景 | 位置建议 | 测试类型 | 输入 | 期望 |
|---|---|---|---|---|
| 触发 Warning/Progress 不崩溃 | `web/` demo + Playwright | e2e | 运行脚本触发记录 | 页面不中断；后续命令仍可执行；console 无 wasm panic |
| HostCall Prompt 输入不吞字符 | 同上 | e2e | `Read-Host` + 先输入别的命令 | prompt 读取值准确；前一次命令不会泄漏 |

---

## 实施顺序建议（最小可交付）
1. Rust：先把 `WarningRecord` 这条 panic 链路修到 **永不 panic**（即便先降级为 Unsupported 文本）。
2. Rust：补齐 Progress/Verbose/Debug/Information 的降级处理。
3. WASM：导出统一事件模型（最小：OutputText + Unsupported）。
4. HostCall：逐个替换 `format!`，确保 tsify + camelCase，并收紧 TS handler 类型。
5. 用 E2E 恢复/新增覆盖（warning/progress），作为发布门槛。

---

## 备注：关于 “让它在 prod thrive”
- 真正的生产稳定性依赖于：**协议层永不 panic + 事件模型稳定 + 类型系统不漏 any**。
- UI/terminal 的“显示效果”可以迭代，但协议层的“不断线/不中断”必须第一优先级。

