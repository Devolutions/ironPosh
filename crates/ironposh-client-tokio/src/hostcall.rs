use futures::StreamExt;
use ironposh_client_async::HostResponse;
use ironposh_client_core::host::HostCall;
use ironposh_terminal::TerminalOp;
use tracing::{error, warn};

use crate::types::TerminalOperation;

/// Process a single host call and return the submission
async fn process_host_call(
    host_call: HostCall,
    ui_tx: &tokio::sync::mpsc::Sender<TerminalOperation>,
) -> Result<ironposh_client_core::host::Submission, anyhow::Error> {
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
        _ => {
            warn!(method = %host_call.method_name(), "unhandled host call");
            // For other host calls, we need to handle them generically
            // Extract the transport and accept with appropriate default result
            match host_call.method_name() {
                name if name.contains("Get") => {
                    // For Get methods, try to return a default string
                    match host_call {
                        HostCall::GetVersion { transport } => {
                            let (_, rt) = transport.into_parts();
                            rt.accept_result("1.0".to_string())
                        }
                        HostCall::GetInstanceId { transport } => {
                            let (_, rt) = transport.into_parts();
                            rt.accept_result(uuid::Uuid::new_v4())
                        }
                        HostCall::GetCurrentCulture { transport } => {
                            let (_, rt) = transport.into_parts();
                            rt.accept_result("en-US".to_string())
                        }
                        HostCall::GetCurrentUICulture { transport } => {
                            let (_, rt) = transport.into_parts();
                            rt.accept_result("en-US".to_string())
                        }
                        _ => {
                            warn!(method = %host_call.method_name(), "unhandled Get host call");
                            // This requires more specific handling based on return type
                            // For now, we'll handle specific cases above
                            panic!("Unhandled host call: {}", host_call.method_name())
                        }
                    }
                }
                _ => {
                    warn!(method = %host_call.method_name(), "unhandled non-Get host call");
                    // For other calls, try to return unit
                    panic!("Unhandled host call: {}", host_call.method_name())
                }
            }
        }
    };

    Ok(submission)
}

/// Handle host calls from PowerShell in a loop, implementing the UI operations
pub async fn handle_host_calls(
    mut host_call_rx: futures::channel::mpsc::UnboundedReceiver<HostCall>,
    submitter: ironposh_client_async::HostSubmitter,
    ui_tx: tokio::sync::mpsc::Sender<TerminalOperation>,
) {
    while let Some(host_call) = host_call_rx.next().await {
        let scope = host_call.scope();
        let call_id = host_call.call_id();

        match process_host_call(host_call, &ui_tx).await {
            Ok(submission) => {
                // Submit the response back
                if let Err(e) = submitter
                    .submit(HostResponse {
                        call_id,
                        scope,
                        submission,
                    })
                    .await
                {
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
