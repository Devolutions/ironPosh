# Host Calls (PSRP) — Quick Reference

This folder is for LLM-oriented documentation and test procedures.

This document describes the main PSRP host calls we care about for the **tokio terminal client** (`ironposh-client-tokio`) and how they map to real PowerShell behaviors.

## Window Title

### `GetWindowTitle` (ID 41)
- Meaning: Read the current host window title.
- Typical triggers:
  - `$Host.UI.RawUI.WindowTitle`
  - scripts that save/restore the title

### `SetWindowTitle` (ID 42)
- Meaning: Set the current host window title.
- Typical triggers:
  - `$Host.UI.RawUI.WindowTitle = "..."` (often used for progress/status)

## Window/Buffer Size

### `GetBufferSize` (ID 37)
- Meaning: Buffer width/height (buffer can be larger than the visible window).
- Typical triggers:
  - `$Host.UI.RawUI.BufferSize`
  - formatting logic that assumes scrollback/buffer height exists

### `GetWindowSize` (ID 39)
- Meaning: Visible window width/height.
- Typical triggers:
  - `$Host.UI.RawUI.WindowSize`
  - `Format-Table`, `Out-String`, progress rendering

### `GetMaxWindowSize` (ID 43)
- Meaning: Max window size given current buffer constraints.
- Typical triggers:
  - `$Host.UI.RawUI.MaxWindowSize`

### `GetMaxPhysicalWindowSize` (ID 44)
- Meaning: Max physical window size supported by the display/host.
- Typical triggers:
  - `$Host.UI.RawUI.MaxPhysicalWindowSize`

## Colors

### `GetForegroundColor` (ID 27) / `SetForegroundColor` (ID 28)
- Meaning: Get/set default foreground color for subsequent output.
- Typical triggers:
  - `$Host.UI.RawUI.ForegroundColor`
  - `$Host.UI.RawUI.ForegroundColor = [ConsoleColor]::Cyan`

### `GetBackgroundColor` (ID 29) / `SetBackgroundColor` (ID 30)
- Meaning: Get/set default background color for subsequent output.
- Typical triggers:
  - `$Host.UI.RawUI.BackgroundColor`
  - `$Host.UI.RawUI.BackgroundColor = [ConsoleColor]::Black`

## Buffer Contents / Scrolling

### `SetBufferContents2` (ID 49)
- Meaning: Fill a rectangular region with a single `BufferCell` (char + fg/bg + type).
- Typical triggers:
  - `$Host.UI.RawUI.SetBufferContents($rect, $cell)`
- Typical UI effect:
  - clears regions, draws blocks/panels, basic TUI rendering

### `ScrollBufferContents` (ID 51)
- Meaning: Move a rectangular region to another location; fill vacated area.
- Typical triggers:
  - `$Host.UI.RawUI.ScrollBufferContents($source, $destination, $clip, $fill)`
- Typical UI effect:
  - partial scrolling inside TUIs, smooth reflow

## Keyboard Input

### `ReadKey` (ID 46)
- Meaning: Read a single key press; often used for “press any key”, menu navigation, escape/cancel.
- Typical triggers:
  - `$Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")`

### `GetKeyAvailable` (ID 45)
- Meaning: Non-blocking check whether a key is waiting in the input buffer.
- Typical triggers:
  - `if ($Host.UI.RawUI.KeyAvailable) { ... }`

### `FlushInputBuffer` (ID 47)
- Meaning: Drop any pending key presses.
- Typical triggers:
  - `$Host.UI.RawUI.FlushInputBuffer()`

## Line/Interactive Prompts

### `ReadLine` (ID 11)
- Meaning: Read a line from the user.
- Typical triggers:
  - `Read-Host "Prompt"`
  - `$Host.UI.ReadLine()`

### `Prompt` (ID 23)
- Meaning: Prompt for multiple fields (returns a dictionary).
- Typical triggers:
  - calling functions/cmdlets with mandatory parameters (PowerShell prompts for missing params)

### `PromptForChoice` (ID 26)
- Meaning: Show a choice menu, return selected index.
- Typical triggers:
  - `Remove-Item something -Confirm`
  - `$Host.UI.PromptForChoice(...)`

### `PromptForCredential1` / `PromptForCredential2` (IDs 24/25)
- Meaning: Prompt for username/password, return `PSCredential`.
- Typical triggers:
  - `Get-Credential`
  - `$Host.UI.PromptForCredential(...)`

