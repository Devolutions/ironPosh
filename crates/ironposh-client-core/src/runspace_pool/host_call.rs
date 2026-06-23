//! Pure host-call construction and classification helpers extracted from
//! `runspace_pool::pool`.
//!
//! These functions are behavior-preserving and free of any `RunspacePool`
//! state: they only parse a `PsValue` into a [`HostCall`] and classify an
//! existing [`HostCall`]. Anything that needs pool internals (fragmenter, id,
//! shell, the `pending_host_calls` queue) stays in `pool.rs`.

use tracing::debug;
use uuid::Uuid;

use crate::{
    PwshCoreError,
    host::{HostCall, HostCallScope},
};
use ironposh_psrp::PsValue;

/// Parse a `PipelineHostCall` `PsValue` into a [`HostCall`] scoped to a pipeline.
///
/// This is the pure parsing body previously inlined in
/// `RunspacePool::handle_pipeline_host_call`; it does not touch pool state.
pub(super) fn pipeline_host_call_from(
    ps_value: PsValue,
    stream_name: &str,
    command_id: Option<&Uuid>,
) -> Result<HostCall, PwshCoreError> {
    let PsValue::Object(pipeline_host_call) = ps_value else {
        return Err(PwshCoreError::InvalidResponse(
            "Expected PipelineHostCall as PsValue::Object".into(),
        ));
    };

    let pipeline_host_call = ironposh_psrp::PipelineHostCall::try_from(pipeline_host_call)?;

    debug!(
        ?pipeline_host_call,
        stream_name = stream_name,
        command_id = ?command_id,
        method = ?pipeline_host_call.method,
        parameters = ?pipeline_host_call.parameters,
        "Received PipelineHostCall"
    );

    // Question: Can we have a Optional command id here?
    let Some(command_id) = command_id else {
        return Err(PwshCoreError::InvalidResponse(
            "Expected command_id to be Some".into(),
        ));
    };

    let scope = HostCallScope::Pipeline {
        command_id: command_id.to_owned(),
    };

    HostCall::try_from_pipeline(scope, pipeline_host_call).map_err(|e| {
        PwshCoreError::InvalidResponse(format!("Failed to parse host call: {e}").into())
    })
}

/// Classify whether a host call requires a PSRP session key to be established
/// before it can be answered (because it transports secure-string data).
pub(super) fn needs_session_key(host_call: &HostCall) -> bool {
    match host_call {
        HostCall::ReadLineAsSecureString { .. }
        | HostCall::PromptForCredential1 { .. }
        | HostCall::PromptForCredential2 { .. } => true,
        HostCall::Prompt { transport } => {
            let (_, _, fields) = &transport.params;
            fields
                .iter()
                .any(|f| f.parameter_type.contains("SecureString"))
        }
        _ => false,
    }
}
