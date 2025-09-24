use anyhow::Result;
use ironposh_client_core::host::{HostCall, Submission};
use tracing::warn;

/// Handle host calls from PowerShell, implementing basic responses
///
/// This module provides host call handling for the async PowerShell client.
/// Most host calls are stubbed out with warnings since they require UI integration.
pub fn handle_host_call(host_call: HostCall) -> Result<Submission> {
    let submission = match host_call {
        HostCall::GetName { transport } => {
            let (_params, result_transport) = transport.into_parts();
            let host_name = "PowerShell-Host".to_string();
            result_transport.accept_result(host_name)
        }
        HostCall::SetCursorPosition { transport } => {
            let (params, result_transport) = transport.into_parts();
            let xy = params.0;
            let x = xy.x.clamp(0, u16::MAX as i32) as u16;
            let y = xy.y.clamp(0, u16::MAX as i32) as u16;
            warn!(
                "SetCursorPosition not implemented in async client: ({}, {})",
                x, y
            );
            result_transport.accept_result(())
        }
        HostCall::SetBufferContents1 { transport } => {
            let (params, result_transport) = transport.into_parts();
            let _rect = params.0;
            let _cell = params.1;
            warn!("SetBufferContents1 not implemented in async client");
            result_transport.accept_result(())
        }
        HostCall::WriteProgress { transport } => {
            let (_params, result_transport) = transport.into_parts();
            result_transport.accept_result(())
        }
        _ => {
            warn!("Unhandled host call type: {}", host_call.method_name());
            todo!("Handle other host call types")
        }
    };
    Ok(submission)
}
