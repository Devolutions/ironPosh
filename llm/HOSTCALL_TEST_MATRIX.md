# Host Call Test Matrix (Real PowerShell)

Goal: for each PSRP host call below, we have a **concrete PowerShell trigger** to run inside the tokio client and a **visible expectation** (“I should see …”).

Source of truth for method IDs/signatures: `D:\ironwinrm\crates\ironposh-client-core\src\host\host_call.rs`.

## Test Setup (Tokio Terminal Client)

1. Start the tokio client (example):
   - `cargo run -p ironposh-client-tokio -- -v -a ntlm`
2. Enable file logging (recommended for debugging hostcall deadlocks):
   - `IRONPOSH_TOKIO_LOG_FILE=D:\ironwinrm\logs\ironposh-client-tokio.trace.log`
   - `RUST_LOG=ironposh_client_tokio=trace,ironposh_client_core=trace,ironposh_async=info,ironposh_psrp=info`
3. Run the commands in the “Trigger” column **inside the tokio client prompt**.

Notes:
- Some host methods are **not supported by some remoting servers** (they may error with “Remote host method … is not implemented.”). In that case, the trigger still documents what *should* exercise the call on a capable server.
- For hostcalls without a strong visual signal, the acceptance criteria is: **trace log contains the received hostcall**.

## Aggressive End-to-End Refactor Plan (Everything, Done Properly)

This is the “tear down and rebuild” plan to implement *all* MS-PSRP host methods end-to-end, including true RawUI semantics and event-driven input so PSReadLine can run reliably.

### Phase A — Inventory & Spec (no code behavior change)

Deliverables:
- A complete hostcall inventory (1–56): parameters, return, blocking behavior, ordering requirements, UI-thread requirements, and whether it is droppable/coalescable.
- A complete test matrix (this document), including a “trigger or best-available trigger” per call.

### Phase B — New HostUI Architecture (break up responsibilities)

Decisions / components:
- `HostEngine` (async task): pure semantics of Host/HostUI/RawUI, no direct stdout writes.
- `TerminalBackend` (UI task): owns real terminal IO (crossterm), event loop, resize events, and rendering.
- `ScreenModel` (data model): authoritative RawUI state + 2D cell buffer. HostCalls mutate this model; renderer diffs it to the real terminal.
- All “blocking” hostcalls are implemented via request/ack (UI must confirm completion to HostEngine).
- All “high frequency” hostcalls (Progress, repeated cursor moves, redraw loops) use coalescing + rate limiting to avoid backpressure deadlocks.

### Phase C — Input Pipeline (required for ReadKey/Prompt/PSReadLine)

Deliverables:
- Unified key event queue from crossterm event stream.
- Line editor on top of key events (for ReadLine/Prompt/Choice/Credential).
- `KeyAvailable/FlushInputBuffer/ReadKey` correctness with modifiers.
- Resize handling (update RawUI sizes + inform server via host defaults where applicable).

### Phase D — Full RawUI Semantics (required for “real console” behavior)

Deliverables:
- Correct implementation of cursor position, window position, buffer/window sizes, scrolling/moving regions, reading buffer contents, and fill/clear semantics.
- Renderer that can move cursor and draw regions without corrupting prompt/input.

### Phase E — E2E Harness

Deliverables:
- Automated PTY-driven tests: spawn client, run triggers, assert on terminal snapshot + internal state dump + trace logs.
- “PSReadLine enabled” integration test and “dynamic prompt works” test.

## Complete HostCall Test Matrix (MS-PSRP 1–56)

### Host methods (1–10)

| HostCall | ID | Trigger (inside tokio client) | What I should see (correct host) | Verify by |
|---|---:|---|---|---|
| GetName | 1 | `$Host.Name` | A host name string. | Screen |
| GetVersion | 2 | `$Host.Version` | A version string / version object rendered. | Screen |
| GetInstanceId | 3 | `$Host.InstanceId` | A GUID. | Screen |
| GetCurrentCulture | 4 | `$Host.CurrentCulture` | A culture (e.g. `en-US`). | Screen |
| GetCurrentUICulture | 5 | `$Host.CurrentUICulture` | A UI culture (e.g. `en-US`). | Screen |
| SetShouldExit | 6 | `exit 42` | Client exits; exit code should be `42`. | Process exit |
| EnterNestedPrompt | 7 | `$Host.EnterNestedPrompt()` | A nested prompt appears (often `>>` / `NESTED>`); commands still run; `exit` leaves nested prompt. | Screen + Log |
| ExitNestedPrompt | 8 | (from inside nested prompt) `exit` OR `$Host.ExitNestedPrompt()` | Nested prompt ends and returns to normal prompt. | Screen + Log |
| NotifyBeginApplication | 9 | `cmd /c echo IRONPOSH_NATIVE_BEGIN` | Native output prints; trace log contains `NotifyBeginApplication`. | Log |
| NotifyEndApplication | 10 | `cmd /c echo IRONPOSH_NATIVE_END` | Native output prints; trace log contains `NotifyEndApplication`. | Log |

### UI methods (11–26)

| HostCall | ID | Trigger (inside tokio client) | What I should see (correct host) | Verify by |
|---|---:|---|---|---|
| ReadLine | 11 | `$Host.UI.ReadLine()` | Blocks for input; typed text is echoed; returns your line. | Screen |
| ReadLineAsSecureString | 12 | `$Host.UI.ReadLineAsSecureString()` | Blocks for input; input is not echoed (or masked); returns a SecureString. | Screen |
| Write1 | 13 | `$Host.UI.Write("IRONPOSH_WRITE1")` | Text prints without a newline. | Screen |
| Write2 | 14 | `$Host.UI.Write([ConsoleColor]::Red,[ConsoleColor]::Black,"IRONPOSH_WRITE2")` | Colored text prints without newline. | Screen |
| WriteLine1 | 15 | `$Host.UI.WriteLine()` | Prints a blank line (newline). | Screen |
| WriteLine2 | 16 | `$Host.UI.WriteLine("IRONPOSH_WRITELINE2")` | Prints text + newline. | Screen |
| WriteLine3 | 17 | `$Host.UI.WriteLine([ConsoleColor]::Green,[ConsoleColor]::Black,"IRONPOSH_WRITELINE3")` | Prints colored text + newline. | Screen |
| WriteErrorLine | 18 | `$Host.UI.WriteErrorLine("IRONPOSH_ERRORLINE")` | An error-colored line (often red) appears. | Screen |
| WriteDebugLine | 19 | `$Host.UI.WriteDebugLine("IRONPOSH_DEBUGLINE")` | A debug line appears (may be hidden depending on `$DebugPreference`). | Screen |
| WriteProgress | 20 | `1..10 | % { Write-Progress -Activity "IRONPOSH" -Status "step $_" -PercentComplete ($_*10); Start-Sleep -Milliseconds 150 }; Write-Progress -Activity "IRONPOSH" -Completed` | A progress UI appears and updates; it clears on completion. | Screen |
| WriteVerboseLine | 21 | `$Host.UI.WriteVerboseLine("IRONPOSH_VERBOSELINE")` | A verbose line appears (may be hidden depending on `$VerbosePreference`). | Screen |
| WriteWarningLine | 22 | `$Host.UI.WriteWarningLine("IRONPOSH_WARNINGLINE")` | A warning-colored line appears. | Screen |
| Prompt | 23 | `function Test-Prompt { param([Parameter(Mandatory)][string]$Name,[Parameter(Mandatory)][int]$Age) }; Test-Prompt` | Prompts for missing mandatory parameters; returns after you fill them. | Screen |
| PromptForCredential1 | 24 | `$Host.UI.PromptForCredential("IRONPOSH","Enter creds","", "")` | Credential prompt appears; returns PSCredential. | Screen |
| PromptForCredential2 | 25 | `$Host.UI.PromptForCredential("IRONPOSH","Enter creds","", "", 0, 0)` | Credential prompt appears; returns PSCredential. | Screen |
| PromptForChoice | 26 | `$choices=@([System.Management.Automation.Host.ChoiceDescription]::new("&Yes",""),[System.Management.Automation.Host.ChoiceDescription]::new("&No","")); $Host.UI.PromptForChoice("IRONPOSH","Pick one",$choices,1)` | A choice prompt appears; returns selected index. | Screen |

### RawUI methods (27–51)

| HostCall | ID | Trigger (inside tokio client) | What I should see (correct host) | Verify by |
|---|---:|---|---|---|
| GetForegroundColor | 27 | `$Host.UI.RawUI.ForegroundColor` | Prints current foreground color. | Screen |
| SetForegroundColor | 28 | `$Host.UI.RawUI.ForegroundColor="Green"; $Host.UI.RawUI.ForegroundColor` | Foreground changes for subsequent output. | Screen |
| GetBackgroundColor | 29 | `$Host.UI.RawUI.BackgroundColor` | Prints current background color. | Screen |
| SetBackgroundColor | 30 | `$Host.UI.RawUI.BackgroundColor="Black"; $Host.UI.RawUI.BackgroundColor` | Background changes for subsequent output. | Screen |
| GetCursorPosition | 31 | `$Host.UI.RawUI.CursorPosition` | Prints cursor coordinates. | Screen |
| SetCursorPosition | 32 | `$Host.UI.RawUI.CursorPosition=[System.Management.Automation.Host.Coordinates]::new(0,0); "TOPLEFT"` | The word appears at top-left (or as close as possible). | Screen |
| GetWindowPosition | 33 | `$Host.UI.RawUI.WindowPosition` | Prints window position. | Screen |
| SetWindowPosition | 34 | `$Host.UI.RawUI.WindowPosition=[System.Management.Automation.Host.Coordinates]::new(0,0); $Host.UI.RawUI.WindowPosition` | Either changes window origin or throws a clear error. | Screen |
| GetCursorSize | 35 | `$Host.UI.RawUI.CursorSize` | Prints cursor size (percentage). | Screen |
| SetCursorSize | 36 | `$Host.UI.RawUI.CursorSize=25; $Host.UI.RawUI.CursorSize` | Cursor size changes or throws consistently. | Screen |
| GetBufferSize | 37 | `$Host.UI.RawUI.BufferSize` | Returns buffer size. | Screen |
| SetBufferSize | 38 | `$Host.UI.RawUI.BufferSize=[System.Management.Automation.Host.Size]::new(120,300); $Host.UI.RawUI.BufferSize` | Buffer resizes or throws consistently. | Screen |
| GetWindowSize | 39 | `$Host.UI.RawUI.WindowSize` | Returns window size. | Screen |
| SetWindowSize | 40 | `$Host.UI.RawUI.WindowSize=[System.Management.Automation.Host.Size]::new(120,40); $Host.UI.RawUI.WindowSize` | Window resizes or throws consistently. | Screen |
| GetWindowTitle | 41 | `$Host.UI.RawUI.WindowTitle` | Prints title string. | Screen |
| SetWindowTitle | 42 | `$Host.UI.RawUI.WindowTitle="IRONPOSH TITLE TEST"; $Host.UI.RawUI.WindowTitle` | Title changes; getter returns same string. | Screen |
| GetMaxWindowSize | 43 | `$Host.UI.RawUI.MaxWindowSize` | Returns max window size. | Screen |
| GetMaxPhysicalWindowSize | 44 | `$Host.UI.RawUI.MaxPhysicalWindowSize` | Returns max physical window size. | Screen |
| GetKeyAvailable | 45 | `if ($Host.UI.RawUI.KeyAvailable) { "HAS_KEY" } else { "NO_KEY" }` | `HAS_KEY` only when a key is pending; does not consume the key. | Screen |
| ReadKey | 46 | `Write-Host "Press a key..."; $k=$Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown"); $k` | Blocks until key; returns KeyInfo (VK, char, modifiers). | Screen |
| FlushInputBuffer | 47 | `Write-Host "Type keys now..."; Start-Sleep 2; $Host.UI.RawUI.FlushInputBuffer(); $Host.UI.RawUI.KeyAvailable` | After flush, KeyAvailable is false. | Screen |
| SetBufferContents1 | 48 | `Clear-Host; "after clear"` | Screen clears; then prints `after clear`. | Screen |
| SetBufferContents2 | 49 | `$rect=[System.Management.Automation.Host.Rectangle]::new(0,0,20,5); $cell=[System.Management.Automation.Host.BufferCell]::new(' ', 'White', 'Blue', 'Complete'); $Host.UI.RawUI.SetBufferContents($rect,$cell); "OK"` | Rect region fills; `OK` prints. | Screen |
| GetBufferContents | 50 | `Clear-Host; "ABC"; $rect=[System.Management.Automation.Host.Rectangle]::new(0,0,2,0); $Host.UI.RawUI.GetBufferContents($rect) | Out-String` | Returned BufferCells contain `A`, `B`, `C` with correct attributes. | Screen |
| ScrollBufferContents | 51 | `Clear-Host; 1..30 | % { "LINE $_" }; $src=[System.Management.Automation.Host.Rectangle]::new(0,0,20,5); $dst=[System.Management.Automation.Host.Coordinates]::new(0,10); $clip=[System.Management.Automation.Host.Rectangle]::new(0,0,160,45); $fill=[System.Management.Automation.Host.BufferCell]::new(' ', 'White', 'Black', 'Complete'); $Host.UI.RawUI.ScrollBufferContents($src,$dst,$clip,$fill); "OK"` | The source block moves to destination; vacated area is filled; `OK` prints. | Screen |

### Interactive session methods (52–56)

These are behind interfaces:
- `IHostSupportsInteractiveSession`
- `IHostUISupportsMultipleChoiceSelection`

| HostCall | ID | Trigger (inside tokio client) | What I should see (correct host) | Verify by |
|---|---:|---|---|---|
| PushRunspace | 52 | `$ih=[System.Management.Automation.Host.IHostSupportsInteractiveSession]$Host; $ih.PushRunspace($ih.Runspace); "OK"` | Host pushes a runspace; client stays interactive; `OK` prints. | Screen + Log |
| PopRunspace | 53 | `$ih=[System.Management.Automation.Host.IHostSupportsInteractiveSession]$Host; $ih.PopRunspace(); "OK"` | Returns to previous runspace; `OK` prints. | Screen + Log |
| GetIsRunspacePushed | 54 | `$ih=[System.Management.Automation.Host.IHostSupportsInteractiveSession]$Host; $ih.IsRunspacePushed` | Prints True/False. | Screen |
| GetRunspace | 55 | `$ih=[System.Management.Automation.Host.IHostSupportsInteractiveSession]$Host; $ih.Runspace | Out-String` | Prints runspace object info. | Screen |
| PromptForChoiceMultipleSelection | 56 | `$mh=[System.Management.Automation.Host.IHostUISupportsMultipleChoiceSelection]$Host.UI; $choices=@([System.Management.Automation.Host.ChoiceDescription]::new("&A",""),[System.Management.Automation.Host.ChoiceDescription]::new("&B","")); $mh.PromptForChoice("T","Pick one or more",$choices,@(0))` | Multi-select UI appears; returns selected indexes array. | Screen |

## Notes / Tips

- For “dynamic prompt / PSReadLine”: once `ReadKey`, cursor/buffer ops, and size handling are correct, PSReadLine should stop disabling itself. A good smoke test is: arrow keys history navigation, editable command line, and prompt repaint without corruption.
- If something “hangs”: in the aggressive architecture, *no HostCall should ever wait on an unbounded UI queue*. Use request/ack and/or coalescing.

## Automation note: why `-c/--command` matters for a PTY-driven matrix

The test matrix is easiest to automate when each HostCall trigger can be executed as a **single, self-contained command** that:
- runs without relying on an interactive REPL prompt, and
- exits on its own so a harness can move to the next ID.

`ironposh-client-tokio` has a `-c/--command` flag, which is the natural way to do this. However, as of now the non-interactive branch is not finished:
- `D:\ironwinrm\crates\ironposh-client-tokio\src\main.rs` uses `unimplemented!(\"{event:?}\")` while consuming the event stream in `-c` mode.

Impact:
- any attempt to run `ironposh-client-tokio.exe -c '<ps command>'` will panic/exit early, so the full 1–56 matrix cannot be batch-executed reliably.

Workaround (until `-c` is fully implemented):
- Use a long-lived interactive session (PTY) and manually/semiautomatically feed commands.
- For “visual-only” calls (e.g., `NotifyBeginApplication` / `NotifyEndApplication`), rely on trace logs to confirm the HostCall occurred.
