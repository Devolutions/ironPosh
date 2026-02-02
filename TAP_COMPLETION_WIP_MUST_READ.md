# Tab Completion (PSRP) - WIP / MUST READ

目标：在 `ironposh-client-tokio` 里按下 Tab 时，像 PowerShell 一样完成补全（尤其是路径补全）。

我们已确认的事实：
- PSRP 没有 “TabCompletion” 专用消息类型。
- 远端补全通过运行脚本 `TabExpansion2 -inputScript <...> -cursorColumn <...>` 实现。
- 返回值是一个对象：`System.Management.Automation.CommandCompletion`（含 `ReplacementIndex` / `ReplacementLength` / `CompletionMatches` 等）。

---

## 设计原则

- 不做协议扩展：只用现有 pipeline invoke。
- 不使用 `Out-String`：必须拿到对象输出（否则变成表格字符串，丢失结构）。
- 索引要按 PowerShell 语义处理：`cursorColumn` / `ReplacementIndex` / `ReplacementLength` 按 “字符串位置” 计数，不能简单当 UTF-8 byte index。
- 先做 MVP：单次 Tab 取第一个候选并替换；再做循环/候选 UI。

---

## 分层实现计划

### 1) `ironposh-psrp`：定义并反序列化补全对象

新增模块（建议）：`crates/ironposh-psrp/src/completion.rs`

实现：
- `CommandCompletion`
  - `current_match_index: i32`
  - `replacement_index: i32`
  - `replacement_length: i32`
  - `completion_matches: Vec<CompletionResult>`
- `CompletionResult`
  - `completion_text: String`
  - `list_item_text: String`
  - `result_type: String`
  - `tool_tip: String`
- `TryFrom<&PsValue>` / `TryFrom<PsValue>` 解析：
  - 顶层是 `PsValue::Object(ComplexObject)`，字段在 `adapted_properties`
  - `CompletionMatches` 是 `Container(List([...]))`，list element 是 `CompletionResult` object

### 2) `ironposh-client-core` / `ironposh-async`：支持 “raw object pipeline”

要求：能发送一个只包含脚本的 pipeline，并返回 `PipelineOutput.data: PsValue`，而不是 Out-String 文本。

### 3) `ironposh-client-tokio`：Tab 键触发补全

输入层：
- 捕获 Tab 键
- 获取当前输入行文本 + 光标位置（本地编辑器状态）

请求层：
- 构造 `TabExpansion2` 脚本（here-string）
- `cursorColumn` 需要从本地 cursor 转换成 PowerShell 语义的索引（建议 UTF-16 code unit offset 映射）
- 发送 raw pipeline，获取第一个对象输出并解析为 `CommandCompletion`

应用层：
- 将 `ReplacementIndex` / `ReplacementLength` 映射回本地 string 的 byte range
- 替换为选中候选的 `completion_text`
- 更新光标到替换后的末尾

交互层（迭代）：
- MVP：单候选自动替换，多候选默认取第一个
- Iteration 2：重复 Tab 轮换候选（保存 completion state）
- Iteration 3：显示候选列表（`list_item_text` + `tool_tip`）

---

## TDD 计划（必须先测再写）

### A) 解析单元测试（`ironposh-psrp`）

- 夹具：保存一份真实的 `CommandCompletion` CLIXML（由本地 `TabExpansion2` + `[PSSerializer]::Serialize` 生成）
- 流程：CLIXML → `PsValue` → `CommandCompletion`
- 断言：
  - `replacement_index` / `replacement_length` 值正确
  - `completion_matches.len() > 0`
  - 任意一条 `CompletionResult` 字段齐全（`CompletionText/ListItemText/ResultType/ToolTip`）

### B) 替换逻辑单元测试（纯字符串）

- 输入：原始输入、cursor、`CommandCompletion`、选择的候选 index
- 输出：新的输入、新的 cursor
- 覆盖：
  - `replacement_length == 0` 的插入场景
  - 多候选轮换
  - UTF-16 offset ↔ UTF-8 byte index 映射边界（至少保证不 panic）

### C) 脚本构造测试

- 确保 here-string 构造不会破坏输入（包含引号/反斜杠/换行）
- `cursorColumn` 正确注入

### D) 端到端冒烟（tokio client）

- 在本地安全目标上跑一次：
  - 输入 `Get-Ser`，按 Tab
  - 期望：输入变为某个 `Get-Serv...` 候选

---

## 完成定义（DoD）

- tokio client Tab 会触发远端 `TabExpansion2`
- 解析到 `CommandCompletion` 对象（不是表格字符串）
- 本地输入缓冲正确替换并更新光标
- 单元测试覆盖解析与替换

