# Host Call Test Matrix (Real PowerShell)

Goal: for each host call below, we have a **concrete PowerShell command** to run inside the tokio client and a **visible expectation** (“I should see …”).

## Test Setup (Tokio Terminal Client)

1. Start the tokio client (example):
   - `cargo run -p ironposh-client-tokio -- -v -a ntlm`
2. Ensure file logging is enabled if you want traces:
   - `IRONPOSH_TOKIO_LOG_FILE=D:\ironwinrm\logs\ironposh-client-tokio.trace.log`
   - `RUST_LOG=ironposh_client_tokio=trace,ironposh_client_core=trace,ironposh_async=info,ironposh_psrp=info`
3. Run the commands in the “PowerShell command to run” column **inside the tokio client prompt**.

## Matrix

Legend (Tokio client status):
- ✅ Implemented: handled in `ironposh-client-tokio`
- ⚠️ Partial: handled but may be stubby/approx
- ❌ Not implemented: currently expected to panic / crash or behave incorrectly

| Host call | ID | PowerShell command to run (inside tokio client) | What you should see (expected behavior) | Tok io status | Notes |
|---|---:|---|---|:--:|---|
| GetWindowTitle | 41 | `$Host.UI.RawUI.WindowTitle` | Prints current title text (string). | ❌ | Not handled in tokio hostcall handler today. |
| SetWindowTitle | 42 | `$Host.UI.RawUI.WindowTitle = "IRONPOSH TITLE TEST"` | Host window title changes to `IRONPOSH TITLE TEST`. | ❌ | Needs bridging to host terminal title APIs. |
| GetBufferSize | 37 | `$Host.UI.RawUI.BufferSize` | Prints `Width`/`Height`. | ❌ | Tok io client currently doesn’t respond. |
| GetWindowSize | 39 | `$Host.UI.RawUI.WindowSize` | Prints visible `Width`/`Height`. | ❌ | Should map to current terminal size. |
| GetMaxWindowSize | 43 | `$Host.UI.RawUI.MaxWindowSize` | Prints max `Width`/`Height` (buffer-constrained). | ❌ | Depends on buffer/window modeling. |
| GetMaxPhysicalWindowSize | 44 | `$Host.UI.RawUI.MaxPhysicalWindowSize` | Prints max `Width`/`Height` (physical). | ❌ | Typically host-dependent. |
| GetForegroundColor | 27 | `$Host.UI.RawUI.ForegroundColor` | Prints current foreground color (e.g. `Gray`). | ❌ | Not handled in tokio today. |
| SetForegroundColor | 28 | `$Host.UI.RawUI.ForegroundColor = "Green"; "GREEN_TEXT"` | `GREEN_TEXT` appears in green (or nearest supported). | ❌ | Requires ANSI/SRG mapping + state. |
| GetBackgroundColor | 29 | `$Host.UI.RawUI.BackgroundColor` | Prints current background color. | ❌ | Not handled in tokio today. |
| SetBackgroundColor | 30 | `$Host.UI.RawUI.BackgroundColor = "DarkBlue"; "BG_TEST"` | `BG_TEST` rendered with dark blue background. | ❌ | Requires ANSI/SRG mapping + state. |
| SetBufferContents2 | 49 | `$rect = [System.Management.Automation.Host.Rectangle]::new(0,0,20,5); $cell = [System.Management.Automation.Host.BufferCell]::new(' ', 'White', 'Blue', 'Complete'); $Host.UI.RawUI.SetBufferContents($rect,$cell)` | A colored rectangle appears at the top-left (blue background). | ❌ | Tok io implements `SetBufferContents1` only. |
| ScrollBufferContents | 51 | `$source=[System.Management.Automation.Host.Rectangle]::new(0,0,20,5); $dest=[System.Management.Automation.Host.Coordinates]::new(0,10); $clip=[System.Management.Automation.Host.Rectangle]::new(0,0,120,40); $fill=[System.Management.Automation.Host.BufferCell]::new(' ', 'White', 'Black', 'Complete'); $Host.UI.RawUI.ScrollBufferContents($source,$dest,$clip,$fill)` | The rectangular region moves; vacated area is filled with spaces. | ❌ | Requires a “move region” terminal op. |
| ReadKey | 46 | `Write-Host "Press a key..."; $k = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown"); $k` | Pauses until key press; then prints key info object. | ❌ | Needs a key-event queue in UI thread. |
| GetKeyAvailable | 45 | `if ($Host.UI.RawUI.KeyAvailable) { "HAS_KEY" } else { "NO_KEY" }` | Prints `NO_KEY` when idle; `HAS_KEY` when a key is queued. | ❌ | Must not consume key events. |
| FlushInputBuffer | 47 | `Write-Host "Type keys now..."; Start-Sleep 2; $Host.UI.RawUI.FlushInputBuffer(); $Host.UI.RawUI.KeyAvailable` | After flush, `KeyAvailable` should be `False`. | ❌ | Needs buffering + flush logic. |
| SetCursorPosition | 32 | `$Host.UI.RawUI.CursorPosition = [System.Management.Automation.Host.Coordinates]::new(0,0); "TOPLEFT"` | Cursor jumps to top-left; text appears there. | ✅ | Implemented (`SetCursorPosition`). |
| SetBufferContents1 (clear) | 48 | `Clear-Host; "after clear"` | Screen clears then prints `after clear`. | ✅ | Implemented with clear-screen fast path. |
| Write / WriteLine (various) | 13–22 | `"A"; Write-Host -NoNewline "B"; "C"` | `B` is not followed by an extra newline; output layout matches real console. | ✅ | Implemented (proper write vs writeline). |
| ReadLine | 11 | `Read-Host "Name"` | Shows prompt `Name:`; accepts input; returns typed string. | ❌ | Tok io uses its own REPL input; hostcall `ReadLine` not implemented. |
| Prompt (mandatory params) | 23 | `function Test-Prompt { param([Parameter(Mandatory)][string]$Name) }; Test-Prompt` | PowerShell prompts for `Name` and waits for input. | ❌ | Requires implementing `Prompt`. |
| PromptForChoice (-Confirm) | 26 | `Remove-Item "$env:TEMP\\ironposh_confirm_test.txt" -ErrorAction SilentlyContinue; Set-Content "$env:TEMP\\ironposh_confirm_test.txt" "x"; Remove-Item "$env:TEMP\\ironposh_confirm_test.txt" -Confirm` | Shows confirmation menu (`[Y] Yes  [N] No ...`), waits for selection. | ❌ | Requires implementing `PromptForChoice`. |
| PromptForCredential | 24/25 | `Get-Credential` | Opens username/password prompt; returns credential object. | ❌ | Requires secure input masking + PSCredential return. |

## Notes / Tips

- Some calls are easiest to observe by **visual side effects** (cursor move, clear, colors, title).
- For unimplemented calls in tokio today, the expected outcome is typically a **panic** with “Unhandled host call …” plus a log entry in the tracing log file.

