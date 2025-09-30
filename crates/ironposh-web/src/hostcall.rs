use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::HostCall;
use tracing::{error, info};

/// This should definately be handled by JS side, but for now we leave it like this so at least the session's loop is not blocked
pub async fn handle_host_calls(
    mut host_call_rx: futures::channel::mpsc::UnboundedReceiver<HostCall>,
    submitter: ironposh_async::HostSubmitter,
) {
    while let Some(host_call) = host_call_rx.next().await {
        let scope = host_call.scope();
        let call_id = host_call.call_id();

        let submission = match host_call {
            HostCall::GetName { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result("PowerShell-Web-Host".to_string())
            }
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
            HostCall::SetCursorPosition { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result(())
            }
            HostCall::SetBufferContents1 { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result(())
            }
            HostCall::WriteProgress { transport } => {
                let (_params, rt) = transport.into_parts();
                rt.accept_result(())
            }
            _ => {
                info!(method = %host_call.method_name(), "unhandled host call, returning empty result");
                continue;
            }
        };

        if let Err(e) = submitter
            .submit(HostResponse {
                call_id,
                scope,
                submission,
            })
            .await
        {
            error!(?e, "failed to submit host call response");
            break;
        }
    }
}
