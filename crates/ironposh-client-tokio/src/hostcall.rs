use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::HostCall;
use ironposh_client_core::host::Size;
use ironposh_terminal::TerminalOp;
use std::sync::Arc;
use tracing::Instrument;
use tracing::{debug, error, trace, warn};

use crate::types::TerminalOperation;

#[derive(Debug)]
pub struct HostUiState {
    pub scrollback_lines: i32,
    pub window_title: String,
    pub foreground_color: i32,
    pub background_color: i32,
}

impl HostUiState {
    pub fn new(scrollback_lines: i32) -> Self {
        Self {
            scrollback_lines,
            window_title: "IronPosh".to_string(),
            foreground_color: 7, // Gray
            background_color: 0, // Black
        }
    }
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
                let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                rt.accept_result(Size {
                    width: cols as i32,
                    height: rows as i32,
                })
            }
            HostCall::GetBufferSize { transport } => {
                let ((), rt) = transport.into_parts();
                let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                let scrollback = { ui_state.lock().await.scrollback_lines };
                rt.accept_result(Size {
                    width: cols as i32,
                    height: rows as i32 + scrollback,
                })
            }
            HostCall::GetMaxPhysicalWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                rt.accept_result(Size {
                    width: cols as i32,
                    height: rows as i32,
                })
            }
            HostCall::GetMaxWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                let scrollback = { ui_state.lock().await.scrollback_lines };
                rt.accept_result(Size {
                    width: cols as i32,
                    height: rows as i32 + scrollback,
                })
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
                let ((_x, _y, text), rt) = transport.into_parts();
                trace!(
                    x = _x,
                    y = _y,
                    text_len = text.len(),
                    newline = false,
                    "host wrote positioned text"
                );
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
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
                let ((_x, _y, text), rt) = transport.into_parts();
                trace!(
                    x = _x,
                    y = _y,
                    text_len = text.len(),
                    newline = true,
                    "host wrote positioned line"
                );
                let _ = ui_tx
                    .send(TerminalOperation::Write {
                        text,
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
            _ => {
                warn!(method = %host_call.method_name(), "unhandled host call");
                // For other host calls, we need to handle them generically
                // Extract the transport and accept with appropriate default result
                match host_call.method_name() {
                    name if name.contains("Get") => {
                        // For Get methods, try to return a default string
                        match host_call {
                            HostCall::GetVersion { transport } => {
                                let ((), rt) = transport.into_parts();
                                rt.accept_result("1.0".to_string())
                            }
                            HostCall::GetInstanceId { transport } => {
                                let ((), rt) = transport.into_parts();
                                rt.accept_result(uuid::Uuid::new_v4())
                            }
                            HostCall::GetCurrentCulture { transport } => {
                                let ((), rt) = transport.into_parts();
                                rt.accept_result("en-US".to_string())
                            }
                            HostCall::GetCurrentUICulture { transport } => {
                                let ((), rt) = transport.into_parts();
                                rt.accept_result("en-US".to_string())
                            }
                            _ => {
                                warn!(method = %host_call.method_name(), "unhandled Get host call");
                                error!(
                                    method = %host_call.method_name(),
                                    method_id = host_call.method_id(),
                                    call_id = host_call.call_id(),
                                    scope = ?host_call.scope(),
                                    "unhandled Get host call; panicking"
                                );
                                // This requires more specific handling based on return type
                                // For now, we'll handle specific cases above
                                panic!("Unhandled host call: {}", host_call.method_name())
                            }
                        }
                    }
                    _ => {
                        warn!(method = %host_call.method_name(), "unhandled non-Get host call");
                        error!(
                            method = %host_call.method_name(),
                            method_id = host_call.method_id(),
                            call_id = host_call.call_id(),
                            scope = ?host_call.scope(),
                            "unhandled host call; panicking"
                        );
                        // For other calls, try to return unit
                        panic!("Unhandled host call: {}", host_call.method_name())
                    }
                }
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

        match process_host_call(host_call, &ui_tx, &ui_state).await {
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
