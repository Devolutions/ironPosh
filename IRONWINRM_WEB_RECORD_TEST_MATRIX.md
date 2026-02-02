# ironwinrm Web（WASM）记录流测试矩阵

目标：验证 `crates/ironposh-web` 在 Web 端不会因为 PSRP record（Warning/Verbose/Debug/Progress/Information 等）而 panic，并且 JS 能拿到足够上下文（pipelineId/stream/messageType 等）用于定位。

> 说明：此矩阵以 CI 的构建方式为准（见 `.github/workflows/ci.yml`）。

---

## 构建/产物检查

| 项 | 命令/动作 | 期望 | 结果 |
|---|---|---|---|
| WASM 构建 | `cd D:\ironwinrm\crates\ironposh-web; wasm-pack build --target web --scope devolutions` | 生成 `crates/ironposh-web/pkg` 且 `.d.ts` 含 `PipelineRecord/WasmPsrpRecord` | PASS |
| 组件构建 | `cd D:\ironwinrm\web\powershell-terminal-component; npm run build:component` | TS 构建通过，Vite 打包成功 | PASS |
| App 构建 | `cd D:\ironwinrm\web\powershell-terminal-app; npm run build` | `tsc && vite build` 通过 | PASS |

---

## 端到端（手动 Playwright）覆盖

环境：
- 使用 `web/powershell-terminal-app` 的页面表单（值来自 `.env` / VITE_*），连接到正在运行的 gateway + WinRM 目标。
- 通过 Playwright 自动输入命令，并在 xterm DOM 中等待 marker 出现（确保事件链路打通且会话未崩）。

| Case | PowerShell 命令 | 期望 Web 端行为 | 结果 |
|---|---|---|---|
| WarningRecord | `Write-Warning '__E2E_WARN__'` | 不 panic；终端可见 marker；应走 `PipelineRecord(kind=warning)` 或等价输出 | PASS |
| VerboseRecord | `Write-Verbose '__E2E_VERBOSE__' -Verbose` | 不 panic；终端可见 marker；应走 `PipelineRecord(kind=verbose)` | PASS |
| DebugRecord | `Write-Debug '__E2E_DEBUG__' -Debug` | 不 panic；终端可见 marker；应走 `PipelineRecord(kind=debug)` | PASS |
| InformationRecord | `Write-Information '__E2E_INFO__'` | 不 panic；终端可见 marker；应走 `PipelineRecord(kind=information)` | PASS |
| ProgressRecord + 完成标记 | `1..5 | % { Write-Progress -Activity 'E2E' -Status $_ -PercentComplete ($_*20); Start-Sleep -Milliseconds 50 }; Write-Output '__E2E_PROGRESS_DONE__'` | 不 panic；终端可见 DONE marker；progress 记录应走 `PipelineRecord(kind=progress)` | PASS |
| 会话继续可用 | `Write-Output '__E2E_AFTER__'` | 前面 record 出现后仍可继续执行命令 | PASS |

---

## 未覆盖/待补充

| Case | 原因 | 建议补充方式 |
|---|---|---|
| `PipelineRecord(kind=unsupported)` | 正常 PowerShell 脚本难以稳定触发“未知 message_type” | 在测试环境注入/回放一段包含未知 message_type 的 PSRP 数据，或增加专用“回放模式”集成测试 |

