use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::HostCall;
use ironposh_terminal::TerminalOp;
use tracing::{debug, error, trace, warn};
use tracing::Instrument;

use crate::types::TerminalOperation;

/// Process a single host call and return the submission
async fn process_host_call(
    host_call: HostCall,
    ui_tx: &tokio::sync::mpsc::Sender<TerminalOperation>,
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

            // Send terminal operations to UI thread
            let _ = ui_tx.send(TerminalOperation::Apply(ops)).await;
            rt.accept_result(())
        }
        HostCall::WriteProgress { transport } => {
            let (_params, rt) = transport.into_parts();
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
                .send(TerminalOperation::Write { text, newline: true })
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
                .send(TerminalOperation::Write { text, newline: true })
                .await;
            rt.accept_result(())
        }
        HostCall::WriteErrorLine { transport } => {
            let ((text,), rt) = transport.into_parts();
            debug!(text_len = text.len(), "host wrote error line");
            let _ = ui_tx
                .send(TerminalOperation::Write { text, newline: true })
                .await;
            rt.accept_result(())
        }
        HostCall::WriteWarningLine { transport } => {
            let ((text,), rt) = transport.into_parts();
            debug!(text_len = text.len(), "host wrote warning line");
            let _ = ui_tx
                .send(TerminalOperation::Write { text, newline: true })
                .await;
            rt.accept_result(())
        }
        HostCall::WriteVerboseLine { transport } => {
            let ((text,), rt) = transport.into_parts();
            debug!(text_len = text.len(), "host wrote verbose line");
            let _ = ui_tx
                .send(TerminalOperation::Write { text, newline: true })
                .await;
            rt.accept_result(())
        }
        HostCall::WriteDebugLine { transport } => {
            let ((text,), rt) = transport.into_parts();
            debug!(text_len = text.len(), "host wrote debug line");
            let _ = ui_tx
                .send(TerminalOperation::Write { text, newline: true })
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

        match process_host_call(host_call, &ui_tx).await {
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
