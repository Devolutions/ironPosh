use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::Coordinates;
use ironposh_client_core::host::HostCall;
use ironposh_client_core::host::Size;
use ironposh_psrp::PsValue;
use ironposh_terminal::TerminalOp;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::Instrument;
use tracing::{debug, error, info, trace, warn};

use crate::types::ReplControl;
use crate::types::{HostUiRequest, HostUiResponse, TerminalOperation};

#[derive(Debug)]
pub struct HostUiState {
    pub scrollback_lines: i32,
    pub window_title: String,
    pub foreground_color: i32,
    pub background_color: i32,
    pub window_position: Coordinates,
    pub cursor_size: i32,
    pub should_exit: Option<i32>,
    pub runspace_stack: Vec<PsValue>,
    pub instance_id: uuid::Uuid,
    pub window_size: Size,
    pub buffer_size: Size,
    pub max_window_size: Size,
    pub max_physical_window_size: Size,
}

impl HostUiState {
    pub fn new(scrollback_lines: i32, cols: u16, rows: u16) -> Self {
        let window_size = Size {
            width: cols as i32,
            height: rows as i32,
        };
        let buffer_size = Size {
            width: cols as i32,
            height: rows as i32 + scrollback_lines,
        };
        Self {
            scrollback_lines,
            window_title: "IronPosh".to_string(),
            foreground_color: 7, // Gray
            background_color: 0, // Black
            window_position: Coordinates { x: 0, y: 0 },
            cursor_size: 25,
            should_exit: None,
            runspace_stack: Vec::new(),
            instance_id: uuid::Uuid::new_v4(),
            window_size,
            buffer_size,
            max_window_size: window_size,
            max_physical_window_size: Size {
                width: cols as i32,
                height: rows as i32,
            },
        }
    }
}

async fn request_ui(
    ui_tx: &tokio::sync::mpsc::Sender<TerminalOperation>,
    request: HostUiRequest,
) -> anyhow::Result<HostUiResponse> {
    let (respond_to, rx) = oneshot::channel();
    ui_tx
        .send(TerminalOperation::HostRequest {
            request,
            respond_to,
        })
        .await
        .map_err(|_| anyhow::anyhow!("UI operation channel closed"))?;
    rx.await.map_err(|_| anyhow::anyhow!("UI request canceled"))
}

fn clamp_console_color(color: i32) -> i32 {
    color.clamp(0, 15)
}

fn sgr_for_foreground(color: i32) -> i32 {
    match clamp_console_color(color) {
        0 => 30,  // Black
        1 => 34,  // DarkBlue
        2 => 32,  // DarkGreen
        3 => 36,  // DarkCyan
        4 => 31,  // DarkRed
        5 => 35,  // DarkMagenta
        6 => 33,  // DarkYellow
        7 => 37,  // Gray
        8 => 90,  // DarkGray
        9 => 94,  // Blue
        10 => 92, // Green
        11 => 96, // Cyan
        12 => 91, // Red
        13 => 95, // Magenta
        14 => 93, // Yellow
        15 => 97, // White
        _ => unreachable!("color is clamped to 0..15"),
    }
}

fn sgr_for_background(color: i32) -> i32 {
    match clamp_console_color(color) {
        0 => 40,   // Black
        1 => 44,   // DarkBlue
        2 => 42,   // DarkGreen
        3 => 46,   // DarkCyan
        4 => 41,   // DarkRed
        5 => 45,   // DarkMagenta
        6 => 43,   // DarkYellow
        7 => 47,   // Gray
        8 => 100,  // DarkGray
        9 => 104,  // Blue
        10 => 102, // Green
        11 => 106, // Cyan
        12 => 101, // Red
        13 => 105, // Magenta
        14 => 103, // Yellow
        15 => 107, // White
        _ => unreachable!("color is clamped to 0..15"),
    }
}

fn ansi_sgr_bytes(codes: &[i32]) -> Vec<u8> {
    let mut s = String::new();
    s.push('\x1b');
    s.push('[');
    for (idx, code) in codes.iter().enumerate() {
        if idx != 0 {
            s.push(';');
        }
        s.push_str(&code.to_string());
    }
    s.push('m');
    s.into_bytes()
}

/// Process a single host call and return the submission
async fn process_host_call(
    host_call: HostCall,
    ui_tx: &tokio::sync::mpsc::Sender<TerminalOperation>,
    repl_control_tx: &tokio::sync::mpsc::Sender<ReplControl>,
    ui_state: &tokio::sync::Mutex<HostUiState>,
) -> Result<ironposh_client_core::host::Submission, anyhow::Error> {
    let span = tracing::trace_span!(
        "process_host_call",
        call_id = host_call.call_id(),
        method_id = host_call.method_id(),
        method = %host_call.method_name(),
        scope = ?host_call.scope(),
    );

    async move {
        let submission = match host_call {
            HostCall::GetName { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result("PowerShell-Host".to_string())
            }
            HostCall::SetCursorPosition { transport } => {
                let (params, rt) = transport.into_parts();
                let xy = params.0;
                let x = xy.x.clamp(0, u16::MAX as i32) as u16;
                let y = xy.y.clamp(0, u16::MAX as i32) as u16;

                trace!(x, y, "host requested cursor position");

                // Send cursor position command to UI thread
                let _ = ui_tx
                    .send(TerminalOperation::Apply(vec![TerminalOp::SetCursor {
                        x,
                        y,
                    }]))
                    .await;
                rt.accept_result(())
            }
            HostCall::SetBufferContents1 { transport } => {
                let (params, rt) = transport.into_parts();
                let rect = params.0;
                let cell = params.1;

                let is_clear = cell.character == ' '
                    && ((rect.left == 0 && rect.top == 0)
                        || (rect.left == -1
                            && rect.top == -1
                            && rect.right == -1
                            && rect.bottom == -1));

                debug!(
                    is_clear,
                    rect = ?rect,
                    ch = %cell.character,
                    fg = cell.foreground,
                    bg = cell.background,
                    "host requested buffer contents update"
                );

                let ops = if is_clear {
                    vec![TerminalOp::ClearScreen]
                } else {
                    vec![TerminalOp::FillRect {
                        left: rect.left.max(0) as u16,
                        top: rect.top.max(0) as u16,
                        right: rect.right.max(0) as u16,
                        bottom: rect.bottom.max(0) as u16,
                        ch: cell.character,
                        fg: cell.foreground as u8,
                        bg: cell.background as u8,
                    }]
                };

                // Never block the PSRP host-call response on UI backpressure.
                if let Err(e) = ui_tx.try_send(TerminalOperation::Apply(ops)) {
                    debug!(
                        error = %e,
                        "dropping SetBufferContents1 terminal op (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::SetBufferContents2 { transport } => {
                let (params, rt) = transport.into_parts();
                let rect = params.0;
                let cell = params.1;

                let is_clear = cell.character == ' '
                    && ((rect.left == 0 && rect.top == 0)
                        || (rect.left == -1
                            && rect.top == -1
                            && rect.right == -1
                            && rect.bottom == -1));

                debug!(
                    is_clear,
                    rect = ?rect,
                    ch = %cell.character,
                    fg = cell.foreground,
                    bg = cell.background,
                    "host requested buffer contents update (SetBufferContents2)"
                );

                let ops = if is_clear {
                    vec![TerminalOp::ClearScreen]
                } else {
                    vec![TerminalOp::FillRect {
                        left: rect.left.max(0) as u16,
                        top: rect.top.max(0) as u16,
                        right: rect.right.max(0) as u16,
                        bottom: rect.bottom.max(0) as u16,
                        ch: cell.character,
                        fg: cell.foreground as u8,
                        bg: cell.background as u8,
                    }]
                };

                // Never block the PSRP host-call response on UI backpressure.
                if let Err(e) = ui_tx.try_send(TerminalOperation::Apply(ops)) {
                    debug!(
                        error = %e,
                        "dropping SetBufferContents2 terminal op (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::WriteProgress { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result(())
            }
            HostCall::GetWindowTitle { transport } => {
                let ((), rt) = transport.into_parts();
                let title = { ui_state.lock().await.window_title.clone() };
                rt.accept_result(title)
            }
            HostCall::SetWindowTitle { transport } => {
                let ((title,), rt) = transport.into_parts();
                debug!(title = %title, "host requested window title change");
                {
                    let mut st = ui_state.lock().await;
                    st.window_title.clone_from(&title);
                }
                // Never block the PSRP host-call response on UI backpressure.
                if let Err(e) = ui_tx.try_send(TerminalOperation::SetWindowTitle { title }) {
                    debug!(error = %e, "dropping SetWindowTitle terminal op (UI queue full/closed)");
                }
                rt.accept_result(())
            }
            HostCall::GetWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                let size = { ui_state.lock().await.window_size };
                rt.accept_result(size)
            }
            HostCall::GetBufferSize { transport } => {
                let ((), rt) = transport.into_parts();
                let size = { ui_state.lock().await.buffer_size };
                rt.accept_result(size)
            }
            HostCall::GetMaxPhysicalWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                let size = { ui_state.lock().await.max_physical_window_size };
                rt.accept_result(size)
            }
            HostCall::GetMaxWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                let size = { ui_state.lock().await.max_window_size };
                rt.accept_result(size)
            }
            HostCall::GetForegroundColor { transport } => {
                let ((), rt) = transport.into_parts();
                let fg = { ui_state.lock().await.foreground_color };
                rt.accept_result(fg)
            }
            HostCall::SetForegroundColor { transport } => {
                let ((color,), rt) = transport.into_parts();
                let color = clamp_console_color(color);
                debug!(color = ?color, "host requested foreground color change");
                {
                    let mut st = ui_state.lock().await;
                    st.foreground_color = color;
                }
                let bytes = ansi_sgr_bytes(&[sgr_for_foreground(color)]);
                // Never block the PSRP host-call response on UI backpressure.
                if let Err(e) =
                    ui_tx.try_send(TerminalOperation::Apply(vec![TerminalOp::FeedBytes(bytes)]))
                {
                    debug!(
                        error = %e,
                        "dropping SetForegroundColor terminal op (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::GetBackgroundColor { transport } => {
                let ((), rt) = transport.into_parts();
                let bg = { ui_state.lock().await.background_color };
                rt.accept_result(bg)
            }
            HostCall::SetBackgroundColor { transport } => {
                let ((color,), rt) = transport.into_parts();
                let color = clamp_console_color(color);
                debug!(color = ?color, "host requested background color change");
                {
                    let mut st = ui_state.lock().await;
                    st.background_color = color;
                }
                let bytes = ansi_sgr_bytes(&[sgr_for_background(color)]);
                // Never block the PSRP host-call response on UI backpressure.
                if let Err(e) =
                    ui_tx.try_send(TerminalOperation::Apply(vec![TerminalOp::FeedBytes(bytes)]))
                {
                    debug!(
                        error = %e,
                        "dropping SetBackgroundColor terminal op (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::Write1 { transport } => {
                let ((text,), rt) = transport.into_parts();
                trace!(text_len = text.len(), newline = false, "host wrote text");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: false,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::Write2 { transport } => {
                // PowerShell calls this overload with explicit colors (foreground, background).
                let ((fg, bg, text), rt) = transport.into_parts();
                let fg = clamp_console_color(fg);
                let bg = clamp_console_color(bg);

                let (prev_foreground, prev_background) = {
                    let st = ui_state.lock().await;
                    (st.foreground_color, st.background_color)
                };
                trace!(
                    foreground = fg,
                    background = bg,
                    text_len = text.len(),
                    newline = false,
                    "host wrote colored text"
                );
                let prefix = format!(
                    "\x1b[{};{}m",
                    sgr_for_foreground(fg),
                    sgr_for_background(bg)
                );
                let suffix = format!(
                    "\x1b[{};{}m",
                    sgr_for_foreground(prev_foreground),
                    sgr_for_background(prev_background)
                );
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text: format!("{prefix}{text}{suffix}"),
                        newline: false,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteLine1 { transport } => {
                let ((), rt) = transport.into_parts();
                trace!(newline = true, "host wrote empty line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text: String::new(),
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteLine2 { transport } => {
                let ((text,), rt) = transport.into_parts();
                trace!(text_len = text.len(), newline = true, "host wrote line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteLine3 { transport } => {
                // PowerShell calls this overload with explicit colors (foreground, background).
                let ((fg, bg, text), rt) = transport.into_parts();
                let fg = clamp_console_color(fg);
                let bg = clamp_console_color(bg);

                let (prev_foreground, prev_background) = {
                    let st = ui_state.lock().await;
                    (st.foreground_color, st.background_color)
                };
                trace!(
                    foreground = fg,
                    background = bg,
                    text_len = text.len(),
                    newline = true,
                    "host wrote colored line"
                );
                let prefix = format!(
                    "\x1b[{};{}m",
                    sgr_for_foreground(fg),
                    sgr_for_background(bg)
                );
                let suffix = format!(
                    "\x1b[{};{}m",
                    sgr_for_foreground(prev_foreground),
                    sgr_for_background(prev_background)
                );
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text: format!("{prefix}{text}{suffix}"),
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteErrorLine { transport } => {
                let ((text,), rt) = transport.into_parts();
                debug!(text_len = text.len(), "host wrote error line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteWarningLine { transport } => {
                let ((text,), rt) = transport.into_parts();
                debug!(text_len = text.len(), "host wrote warning line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteVerboseLine { transport } => {
                let ((text,), rt) = transport.into_parts();
                debug!(text_len = text.len(), "host wrote verbose line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::WriteDebugLine { transport } => {
                let ((text,), rt) = transport.into_parts();
                debug!(text_len = text.len(), "host wrote debug line");
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
                        newline: true,
                    })
                    .await;
                rt.accept_result(())
            }
            HostCall::GetVersion { transport } => {
                let ((), rt) = transport.into_parts();
                rt.accept_result(env!("CARGO_PKG_VERSION").to_string())
            }
            HostCall::GetInstanceId { transport } => {
                let ((), rt) = transport.into_parts();
                let id = { ui_state.lock().await.instance_id };
                rt.accept_result(id)
            }
            HostCall::GetCurrentCulture { transport } => {
                let ((), rt) = transport.into_parts();
                rt.accept_result("en-US".to_string())
            }
            HostCall::GetCurrentUICulture { transport } => {
                let ((), rt) = transport.into_parts();
                rt.accept_result("en-US".to_string())
            }
            HostCall::SetShouldExit { transport } => {
                let ((code,), rt) = transport.into_parts();
                warn!(code, "host requested SetShouldExit");
                {
                    let mut st = ui_state.lock().await;
                    st.should_exit = Some(code);
                }
                let _ = repl_control_tx
                    .send(ReplControl::ShouldExit(code))
                    .await;
                rt.accept_result(())
            }
            HostCall::EnterNestedPrompt { transport } => {
                let ((), rt) = transport.into_parts();
                info!("host requested EnterNestedPrompt");
                let _ = repl_control_tx.send(ReplControl::EnterNestedPrompt).await;
                rt.accept_result(())
            }
            HostCall::ExitNestedPrompt { transport } => {
                let ((), rt) = transport.into_parts();
                info!("host requested ExitNestedPrompt");
                let _ = repl_control_tx.send(ReplControl::ExitNestedPrompt).await;
                rt.accept_result(())
            }
            HostCall::NotifyBeginApplication { transport } => {
                let ((), rt) = transport.into_parts();
                trace!("host requested NotifyBeginApplication");
                rt.accept_result(())
            }
            HostCall::NotifyEndApplication { transport } => {
                let ((), rt) = transport.into_parts();
                trace!("host requested NotifyEndApplication");
                rt.accept_result(())
            }
            HostCall::ReadLine { transport } => {
                let ((), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::ReadLine).await?;
                let HostUiResponse::Line(line) = resp else {
                    return Err(anyhow::anyhow!("unexpected ReadLine UI response: {resp:?}"));
                };
                rt.accept_result(line)
            }
            HostCall::ReadLineAsSecureString { transport } => {
                let ((), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::ReadLineAsSecureString).await?;
                let HostUiResponse::SecureBytes(bytes) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected ReadLineAsSecureString UI response: {resp:?}"
                    ));
                };
                rt.accept_result(bytes)
            }
            HostCall::Prompt { transport } => {
                let ((caption, message, fields), rt) = transport.into_parts();
                let resp = request_ui(
                    ui_tx,
                    HostUiRequest::Prompt {
                        caption,
                        message,
                        fields,
                    },
                )
                .await?;
                let HostUiResponse::PromptResult(map) = resp else {
                    return Err(anyhow::anyhow!("unexpected Prompt UI response: {resp:?}"));
                };
                rt.accept_result(map)
            }
            HostCall::PromptForChoice { transport } => {
                let ((caption, message, choices, default_choice), rt) = transport.into_parts();
                let resp = request_ui(
                    ui_tx,
                    HostUiRequest::PromptForChoice {
                        caption,
                        message,
                        choices,
                        default_choice,
                    },
                )
                .await?;
                let HostUiResponse::Choice(choice) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected PromptForChoice UI response: {resp:?}"
                    ));
                };
                rt.accept_result(choice)
            }
            HostCall::PromptForChoiceMultipleSelection { transport } => {
                let ((caption, message, choices, default_choices), rt) = transport.into_parts();
                let resp = request_ui(
                    ui_tx,
                    HostUiRequest::PromptForChoiceMultipleSelection {
                        caption,
                        message,
                        choices,
                        default_choices,
                    },
                )
                .await?;
                let HostUiResponse::ChoiceMulti(choices) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected PromptForChoiceMultipleSelection UI response: {resp:?}"
                    ));
                };
                rt.accept_result(choices)
            }
            HostCall::PromptForCredential1 { transport } => {
                let ((caption, message, user_name, target_name), rt) = transport.into_parts();
                let resp = request_ui(
                    ui_tx,
                    HostUiRequest::PromptForCredential1 {
                        caption,
                        message,
                        user_name,
                        target_name,
                    },
                )
                .await?;
                let HostUiResponse::Credential(cred) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected PromptForCredential1 UI response: {resp:?}"
                    ));
                };
                rt.accept_result(cred)
            }
            HostCall::PromptForCredential2 { transport } => {
                let ((caption, message, user_name, target_name, allowed_credential_types, options), rt) =
                    transport.into_parts();
                let resp = request_ui(
                    ui_tx,
                    HostUiRequest::PromptForCredential2 {
                        caption,
                        message,
                        user_name,
                        target_name,
                        allowed_credential_types,
                        options,
                    },
                )
                .await?;
                let HostUiResponse::Credential(cred) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected PromptForCredential2 UI response: {resp:?}"
                    ));
                };
                rt.accept_result(cred)
            }
            HostCall::GetCursorPosition { transport } => {
                let ((), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::GetCursorPosition).await?;
                let HostUiResponse::CursorPosition(pos) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected GetCursorPosition UI response: {resp:?}"
                    ));
                };
                rt.accept_result(pos)
            }
            HostCall::GetWindowPosition { transport } => {
                let ((), rt) = transport.into_parts();
                let pos = { ui_state.lock().await.window_position };
                rt.accept_result(pos)
            }
            HostCall::SetWindowPosition { transport } => {
                let ((pos,), rt) = transport.into_parts();
                {
                    let mut st = ui_state.lock().await;
                    st.window_position = pos;
                }
                rt.accept_result(())
            }
            HostCall::GetCursorSize { transport } => {
                let ((), rt) = transport.into_parts();
                let size = { ui_state.lock().await.cursor_size };
                rt.accept_result(size)
            }
            HostCall::SetCursorSize { transport } => {
                let ((size,), rt) = transport.into_parts();
                {
                    let mut st = ui_state.lock().await;
                    st.cursor_size = size;
                }
                rt.accept_result(())
            }
            HostCall::SetBufferSize { transport } => {
                let ((size,), rt) = transport.into_parts();
                let size = Size {
                    width: size.width.max(1),
                    height: size.height.max(1),
                };

                let (window_size, scrollback_rows) = {
                    let mut st = ui_state.lock().await;
                    st.buffer_size = size;
                    st.max_window_size = size;

                    // Keep window size within the buffer.
                    if st.window_size.width > st.buffer_size.width {
                        st.window_size.width = st.buffer_size.width;
                    }
                    if st.window_size.height > st.buffer_size.height {
                        st.window_size.height = st.buffer_size.height;
                    }

                    let scrollback_rows =
                        (st.buffer_size.height - st.window_size.height).max(0) as usize;
                    st.scrollback_lines = scrollback_rows
                        .try_into()
                        .unwrap_or(i32::MAX);
                    (st.window_size, scrollback_rows)
                };

                if let Err(e) = ui_tx.try_send(TerminalOperation::Apply(vec![
                    TerminalOp::Resize {
                        cols: window_size.width.clamp(1, u16::MAX as i32) as u16,
                        rows: window_size.height.clamp(1, u16::MAX as i32) as u16,
                    },
                    TerminalOp::SetScrollback {
                        rows: scrollback_rows,
                    },
                ])) {
                    debug!(
                        error = %e,
                        "dropping SetBufferSize terminal ops (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::SetWindowSize { transport } => {
                let ((size,), rt) = transport.into_parts();
                let size = Size {
                    width: size.width.max(1),
                    height: size.height.max(1),
                };

                let (window_size, scrollback_rows) = {
                    let mut st = ui_state.lock().await;
                    st.window_size = size;
                    if st.window_size.width > st.buffer_size.width {
                        st.buffer_size.width = st.window_size.width;
                    }
                    if st.window_size.height > st.buffer_size.height {
                        st.buffer_size.height = st.window_size.height;
                    }

                    let scrollback_rows =
                        (st.buffer_size.height - st.window_size.height).max(0) as usize;
                    st.scrollback_lines = scrollback_rows
                        .try_into()
                        .unwrap_or(i32::MAX);
                    st.max_window_size = st.buffer_size;
                    (st.window_size, scrollback_rows)
                };

                if let Err(e) = ui_tx.try_send(TerminalOperation::Apply(vec![
                    TerminalOp::Resize {
                        cols: window_size.width.clamp(1, u16::MAX as i32) as u16,
                        rows: window_size.height.clamp(1, u16::MAX as i32) as u16,
                    },
                    TerminalOp::SetScrollback {
                        rows: scrollback_rows,
                    },
                ])) {
                    debug!(
                        error = %e,
                        "dropping SetWindowSize terminal ops (UI queue full/closed)"
                    );
                }
                rt.accept_result(())
            }
            HostCall::GetKeyAvailable { transport } => {
                let ((), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::GetKeyAvailable).await?;
                let HostUiResponse::Bool(v) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected GetKeyAvailable UI response: {resp:?}"
                    ));
                };
                rt.accept_result(v)
            }
            HostCall::ReadKey { transport } => {
                let ((options,), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::ReadKey { options }).await?;
                let HostUiResponse::KeyInfo(k) = resp else {
                    return Err(anyhow::anyhow!("unexpected ReadKey UI response: {resp:?}"));
                };
                rt.accept_result(k)
            }
            HostCall::FlushInputBuffer { transport } => {
                let ((), rt) = transport.into_parts();
                let _ = request_ui(ui_tx, HostUiRequest::FlushInputBuffer).await?;
                rt.accept_result(())
            }
            HostCall::GetBufferContents { transport } => {
                let ((rect,), rt) = transport.into_parts();
                let resp = request_ui(ui_tx, HostUiRequest::GetBufferContents { rect }).await?;
                let HostUiResponse::BufferContents(cells) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected GetBufferContents UI response: {resp:?}"
                    ));
                };
                rt.accept_result(cells)
            }
            HostCall::ScrollBufferContents { transport } => {
                let ((source, destination, _clip, fill), rt) = transport.into_parts();

                let resp = request_ui(ui_tx, HostUiRequest::GetBufferContents { rect: source })
                    .await?;
                let HostUiResponse::BufferContents(cells) = resp else {
                    return Err(anyhow::anyhow!(
                        "unexpected GetBufferContents UI response: {resp:?}"
                    ));
                };

                // Best-effort implementation:
                //  1) capture the source rectangle
                //  2) fill the source rectangle with the provided fill cell
                //  3) redraw the captured contents at the destination coordinates
                //
                // This ignores clipping/overlap edge cases but exercises the end-to-end path.

                let fill_op = TerminalOp::FillRect {
                    left: source.left.max(0) as u16,
                    top: source.top.max(0) as u16,
                    right: source.right.max(0) as u16,
                    bottom: source.bottom.max(0) as u16,
                    ch: fill.character,
                    fg: fill.foreground.clamp(0, 15) as u8,
                    bg: fill.background.clamp(0, 15) as u8,
                };

                if let Err(e) = ui_tx.try_send(TerminalOperation::Apply(vec![fill_op])) {
                    debug!(
                        error = %e,
                        "dropping ScrollBufferContents fill op (UI queue full/closed)"
                    );
                }

                let mut bytes = Vec::new();
                bytes.extend_from_slice(b"\x1b7"); // save cursor

                for (row_idx, row) in cells.iter().enumerate() {
                    let y = destination.y + row_idx as i32;
                    if y < 0 {
                        continue;
                    }
                    let y = y.clamp(0, u16::MAX as i32) as u16;

                    let mut col = destination.x;
                    for cell in row {
                        if col >= 0 {
                            let x = col.clamp(0, u16::MAX as i32) as u16;
                            bytes.extend_from_slice(format!("\x1b[{};{}H", y + 1, x + 1).as_bytes());
                            let fg = sgr_for_foreground(cell.foreground);
                            let bg = sgr_for_background(cell.background);
                            bytes.extend_from_slice(&ansi_sgr_bytes(&[fg, bg]));
                            let mut buf = [0u8; 4];
                            let s = cell.character.encode_utf8(&mut buf);
                            bytes.extend_from_slice(s.as_bytes());
                        }
                        col += 1;
                    }
                }

                bytes.extend_from_slice(b"\x1b[0m\x1b8"); // reset attrs + restore cursor

                if let Err(e) =
                    ui_tx.try_send(TerminalOperation::Apply(vec![TerminalOp::FeedBytes(bytes)]))
                {
                    debug!(
                        error = %e,
                        "dropping ScrollBufferContents draw op (UI queue full/closed)"
                    );
                }

                rt.accept_result(())
            }
            HostCall::PushRunspace { transport } => {
                let ((runspace,), rt) = transport.into_parts();
                {
                    let mut st = ui_state.lock().await;
                    st.runspace_stack.push(runspace);
                }
                rt.accept_result(())
            }
            HostCall::PopRunspace { transport } => {
                let ((), rt) = transport.into_parts();
                {
                    let mut st = ui_state.lock().await;
                    let _ = st.runspace_stack.pop();
                }
                rt.accept_result(())
            }
            HostCall::GetIsRunspacePushed { transport } => {
                let ((), rt) = transport.into_parts();
                let pushed = { !ui_state.lock().await.runspace_stack.is_empty() };
                rt.accept_result(pushed)
            }
            HostCall::GetRunspace { transport } => {
                let ((), rt) = transport.into_parts();
                let runspace = {
                    ui_state
                        .lock()
                        .await
                        .runspace_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| PsValue::from(String::new()))
                };
                rt.accept_result(runspace)
            }
        };

        Ok(submission)
    }
    .instrument(span)
    .await
}

/// Handle host calls from PowerShell in a loop, implementing the UI operations
pub async fn handle_host_calls(
    mut host_call_rx: futures::channel::mpsc::UnboundedReceiver<HostCall>,
    submitter: ironposh_async::HostSubmitter,
    ui_tx: tokio::sync::mpsc::Sender<TerminalOperation>,
    repl_control_tx: tokio::sync::mpsc::Sender<ReplControl>,
    ui_state: Arc<tokio::sync::Mutex<HostUiState>>,
) {
    while let Some(host_call) = host_call_rx.next().await {
        let scope = host_call.scope();
        let call_id = host_call.call_id();
        trace!(
            call_id,
            method_id = host_call.method_id(),
            method = %host_call.method_name(),
            scope = ?scope,
            "received host call"
        );

        match process_host_call(host_call, &ui_tx, &repl_control_tx, &ui_state).await {
            Ok(submission) => {
                // Submit the response back
                if let Err(e) = submitter.submit(HostResponse {
                    call_id,
                    scope,
                    submission,
                }) {
                    error!(error = %e, "failed to submit host call response");
                    break;
                }
            }
            Err(e) => {
                error!(error = %e, "failed to process host call");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_client_core::host::{
        BufferCell, ChoiceDescription, FieldDescription, HostCallScope, KeyInfo, PSCredential,
        Rectangle, Transport,
    };
    use std::collections::HashMap;

    fn scope() -> HostCallScope {
        HostCallScope::RunspacePool
    }

    fn spawn_test_ui(
        mut ui_rx: tokio::sync::mpsc::Receiver<TerminalOperation>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(op) = ui_rx.recv().await {
                if let TerminalOperation::HostRequest {
                    request,
                    respond_to,
                } = op
                {
                    let response = match request {
                        HostUiRequest::ReadLine => HostUiResponse::Line("line".to_string()),
                        HostUiRequest::ReadLineAsSecureString => {
                            HostUiResponse::SecureBytes(vec![1, 2, 3, 4])
                        }
                        HostUiRequest::Prompt { .. } => {
                            HostUiResponse::PromptResult(HashMap::new())
                        }
                        HostUiRequest::PromptForChoice { default_choice, .. } => {
                            HostUiResponse::Choice(default_choice)
                        }
                        HostUiRequest::PromptForChoiceMultipleSelection {
                            default_choices, ..
                        } => HostUiResponse::ChoiceMulti(default_choices),
                        HostUiRequest::PromptForCredential1 { user_name, .. }
                        | HostUiRequest::PromptForCredential2 { user_name, .. } => {
                            HostUiResponse::Credential(PSCredential {
                                user_name,
                                password: vec![9, 9, 9],
                            })
                        }
                        HostUiRequest::ReadKey { .. } => HostUiResponse::KeyInfo(KeyInfo {
                            virtual_key_code: 65,
                            character: 'a',
                            control_key_state: 0,
                            key_down: true,
                        }),
                        HostUiRequest::GetKeyAvailable => HostUiResponse::Bool(false),
                        HostUiRequest::FlushInputBuffer => HostUiResponse::Unit,
                        HostUiRequest::GetCursorPosition => {
                            HostUiResponse::CursorPosition(Coordinates { x: 1, y: 2 })
                        }
                        HostUiRequest::GetBufferContents { rect } => {
                            let w = (rect.right - rect.left + 1).max(0) as usize;
                            let h = (rect.bottom - rect.top + 1).max(0) as usize;
                            let row = vec![
                                BufferCell {
                                    character: 'x',
                                    foreground: 7,
                                    background: 0,
                                    flags: 0,
                                };
                                w.max(1)
                            ];
                            HostUiResponse::BufferContents(vec![row; h.max(1)])
                        }
                    };
                    let _ = respond_to.send(response);
                }
            }
        })
    }

    #[tokio::test]
    async fn hostcalls_are_handled_end_to_end() -> anyhow::Result<()> {
        let (ui_tx, ui_rx) = tokio::sync::mpsc::channel::<TerminalOperation>(128);
        let _ui_handle = spawn_test_ui(ui_rx);

        let (repl_control_tx, mut repl_control_rx) = tokio::sync::mpsc::channel::<ReplControl>(8);

        let ui_state = tokio::sync::Mutex::new(HostUiState::new(2000, 120, 30));

        // Host getters (1-5)
        let _ = process_host_call(
            HostCall::GetName {
                transport: Transport::new(scope(), 1, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetVersion {
                transport: Transport::new(scope(), 2, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetInstanceId {
                transport: Transport::new(scope(), 3, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetCurrentCulture {
                transport: Transport::new(scope(), 4, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetCurrentUICulture {
                transport: Transport::new(scope(), 5, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        // Host setters / lifecycle (6-10)
        let _ = process_host_call(
            HostCall::SetShouldExit {
                transport: Transport::new(scope(), 6, (123,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        assert!(matches!(
            repl_control_rx.recv().await,
            Some(ReplControl::ShouldExit(123))
        ));

        let _ = process_host_call(
            HostCall::EnterNestedPrompt {
                transport: Transport::new(scope(), 7, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        assert!(matches!(
            repl_control_rx.recv().await,
            Some(ReplControl::EnterNestedPrompt)
        ));

        let _ = process_host_call(
            HostCall::ExitNestedPrompt {
                transport: Transport::new(scope(), 8, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        assert!(matches!(
            repl_control_rx.recv().await,
            Some(ReplControl::ExitNestedPrompt)
        ));

        let _ = process_host_call(
            HostCall::NotifyBeginApplication {
                transport: Transport::new(scope(), 9, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::NotifyEndApplication {
                transport: Transport::new(scope(), 10, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        // UI methods (11-26)
        let _ = process_host_call(
            HostCall::ReadLine {
                transport: Transport::new(scope(), 11, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::ReadLineAsSecureString {
                transport: Transport::new(scope(), 12, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::Write1 {
                transport: Transport::new(scope(), 13, ("hi".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::Write2 {
                transport: Transport::new(scope(), 14, (0, 0, "hi".to_string())),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteLine1 {
                transport: Transport::new(scope(), 15, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteLine2 {
                transport: Transport::new(scope(), 16, ("hi".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteLine3 {
                transport: Transport::new(scope(), 17, (0, 0, "hi".to_string())),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteErrorLine {
                transport: Transport::new(scope(), 18, ("err".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteDebugLine {
                transport: Transport::new(scope(), 19, ("dbg".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteProgress {
                transport: Transport::new(
                    scope(),
                    20,
                    (
                        1,
                        ironposh_client_core::host::ProgressRecord {
                            activity: String::new(),
                            status_description: String::new(),
                            current_operation: String::new(),
                            activity_id: 0,
                            parent_activity_id: -1,
                            percent_complete: -1,
                            seconds_remaining: -1,
                            record_type: 0,
                        },
                    ),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteVerboseLine {
                transport: Transport::new(scope(), 21, ("v".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::WriteWarningLine {
                transport: Transport::new(scope(), 22, ("w".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let fields = vec![FieldDescription {
            name: "Name".to_string(),
            label: "Name".to_string(),
            help_message: String::new(),
            is_mandatory: false,
            parameter_type: "System.String".to_string(),
            default_value: None,
        }];
        let _ = process_host_call(
            HostCall::Prompt {
                transport: Transport::new(
                    scope(),
                    23,
                    ("cap".to_string(), "msg".to_string(), fields),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::PromptForCredential1 {
                transport: Transport::new(
                    scope(),
                    24,
                    (
                        "cap".to_string(),
                        "msg".to_string(),
                        "u".to_string(),
                        "t".to_string(),
                    ),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::PromptForCredential2 {
                transport: Transport::new(
                    scope(),
                    25,
                    (
                        "cap".to_string(),
                        "msg".to_string(),
                        "u".to_string(),
                        "t".to_string(),
                        0,
                        0,
                    ),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let choices = vec![ChoiceDescription {
            label: "&Yes".to_string(),
            help_message: String::new(),
        }];
        let _ = process_host_call(
            HostCall::PromptForChoice {
                transport: Transport::new(
                    scope(),
                    26,
                    ("cap".to_string(), "msg".to_string(), choices, 0),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        // Raw UI methods (27-51)
        let _ = process_host_call(
            HostCall::GetForegroundColor {
                transport: Transport::new(scope(), 27, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetForegroundColor {
                transport: Transport::new(scope(), 28, (2,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetBackgroundColor {
                transport: Transport::new(scope(), 29, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetBackgroundColor {
                transport: Transport::new(scope(), 30, (0,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetCursorPosition {
                transport: Transport::new(scope(), 31, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetCursorPosition {
                transport: Transport::new(scope(), 32, (Coordinates { x: 0, y: 0 },)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetWindowPosition {
                transport: Transport::new(scope(), 33, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetWindowPosition {
                transport: Transport::new(scope(), 34, (Coordinates { x: 0, y: 0 },)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetCursorSize {
                transport: Transport::new(scope(), 35, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetCursorSize {
                transport: Transport::new(scope(), 36, (25,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::GetBufferSize {
                transport: Transport::new(scope(), 37, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetBufferSize {
                transport: Transport::new(
                    scope(),
                    38,
                    (Size {
                        width: 120,
                        height: 100,
                    },),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetWindowSize {
                transport: Transport::new(scope(), 39, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetWindowSize {
                transport: Transport::new(
                    scope(),
                    40,
                    (Size {
                        width: 100,
                        height: 20,
                    },),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::GetWindowTitle {
                transport: Transport::new(scope(), 41, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetWindowTitle {
                transport: Transport::new(scope(), 42, ("t".to_string(),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::GetMaxWindowSize {
                transport: Transport::new(scope(), 43, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetMaxPhysicalWindowSize {
                transport: Transport::new(scope(), 44, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let _ = process_host_call(
            HostCall::GetKeyAvailable {
                transport: Transport::new(scope(), 45, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::ReadKey {
                transport: Transport::new(scope(), 46, (0,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::FlushInputBuffer {
                transport: Transport::new(scope(), 47, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let rect = Rectangle {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        let cell = BufferCell {
            character: ' ',
            foreground: 7,
            background: 0,
            flags: 0,
        };
        let _ = process_host_call(
            HostCall::SetBufferContents1 {
                transport: Transport::new(scope(), 48, (rect, cell.clone())),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::SetBufferContents2 {
                transport: Transport::new(scope(), 49, (rect, cell.clone())),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetBufferContents {
                transport: Transport::new(scope(), 50, (rect,)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::ScrollBufferContents {
                transport: Transport::new(
                    scope(),
                    51,
                    (rect, Coordinates { x: 0, y: 0 }, rect, cell),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        // Interactive session methods (52-56)
        let _ = process_host_call(
            HostCall::PushRunspace {
                transport: Transport::new(scope(), 52, (PsValue::from("rs".to_string()),)),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::PopRunspace {
                transport: Transport::new(scope(), 53, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetIsRunspacePushed {
                transport: Transport::new(scope(), 54, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;
        let _ = process_host_call(
            HostCall::GetRunspace {
                transport: Transport::new(scope(), 55, ()),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        let choices = vec![ChoiceDescription {
            label: "&Yes".to_string(),
            help_message: String::new(),
        }];
        let _ = process_host_call(
            HostCall::PromptForChoiceMultipleSelection {
                transport: Transport::new(
                    scope(),
                    56,
                    ("cap".to_string(), "msg".to_string(), choices, vec![0]),
                ),
            },
            &ui_tx,
            &repl_control_tx,
            &ui_state,
        )
        .await?;

        Ok(())
    }
}
