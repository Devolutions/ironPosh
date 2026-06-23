use base64::Engine;
use ironposh_psrp::{MessageType, PsrpMessage, fragmentation};
use ironposh_winrm::soap::SoapEnvelope;
use ironposh_xml::parser::XmlDeserialize;
use tracing::{debug, info, trace, warn};

use super::enums::RunspacePoolState;
use super::pool::RunspacePool;

/// Sibling of [`super::ExpectShellCreated`] for the WSMan Connect path.
///
/// Consumes the ConnectResponse of a disconnected shell and yields an
/// `Opened` pool. The caller must fire the initial pool-stream Receive
/// afterwards to resume the receive loop.
#[derive(Debug)]
pub struct ExpectShellConnected {
    pub(super) runspace_pool: RunspacePool,
}

impl ExpectShellConnected {
    pub fn accept(self, response: &str) -> Result<RunspacePool, crate::PwshCoreError> {
        let Self { mut runspace_pool } = self;

        let parsed = ironposh_xml::parser::parse(response)?;

        let soap_response = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        RunspacePool::fault_to_error(&soap_response)?;

        let connect_response = soap_response
            .body
            .as_ref()
            .connect_response
            .as_ref()
            .ok_or_else(|| {
                crate::PwshCoreError::InvalidResponse("No ConnectResponse found in response".into())
            })?;

        let payload = connect_response
            .as_ref()
            .connect_response_xml
            .as_ref()
            .ok_or_else(|| {
                crate::PwshCoreError::InvalidResponse(
                    "ConnectResponse is missing the connectResponseXml payload".into(),
                )
            })?;

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload.value.as_ref())
            .map_err(|e| {
                crate::PwshCoreError::InvalidResponse(
                    format!("Failed to decode connectResponseXml base64: {e}").into(),
                )
            })?;

        let messages = match runspace_pool.defragmenter.defragment(&decoded)? {
            fragmentation::DefragmentResult::Incomplete => {
                return Err(crate::PwshCoreError::InvalidResponse(
                    "connectResponseXml carried an incomplete PSRP fragment stream".into(),
                ));
            }
            fragmentation::DefragmentResult::Complete(messages) => messages,
        };

        let mut saw_session_capability = false;
        let mut saw_init_data = false;
        for message in messages {
            // L4 typed stream (issue #12): parse once into a typed PsrpMessage and
            // match on the variant instead of re-deriving the type from the wire
            // header and re-parsing per arm.
            let data_len = message.data.len();
            match PsrpMessage::parse(&message)? {
                PsrpMessage::SessionCapability(capability) => {
                    debug!(target: "session", ?capability, "received SessionCapability");
                    runspace_pool.session_capability = Some(capability);
                    saw_session_capability = true;
                }
                PsrpMessage::ApplicationPrivateData(app_data) => {
                    trace!(target: "session", ?app_data, "received ApplicationPrivateData");
                    runspace_pool.application_private_data = Some(app_data);
                }
                PsrpMessage::RunspacePoolInitData(init_data) => {
                    info!(
                        ?init_data,
                        "received RunspacePoolInitData in ConnectResponse"
                    );
                    // Reject implausible server values rather than silently coercing them
                    // (a negative or min>max count indicates protocol corruption).
                    let min = usize::try_from(init_data.min_runspaces).map_err(|_| {
                        crate::PwshCoreError::InvalidResponse(
                            "RunspacePoolInitData MinRunspaces is negative".into(),
                        )
                    })?;
                    let max = usize::try_from(init_data.max_runspaces).map_err(|_| {
                        crate::PwshCoreError::InvalidResponse(
                            "RunspacePoolInitData MaxRunspaces is negative".into(),
                        )
                    })?;
                    if min > max {
                        return Err(crate::PwshCoreError::InvalidResponse(
                            "RunspacePoolInitData MinRunspaces exceeds MaxRunspaces".into(),
                        ));
                    }
                    runspace_pool.min_runspaces = min;
                    runspace_pool.max_runspaces = max;
                    saw_init_data = true;
                }
                PsrpMessage::RunspacePoolState(_) => {
                    // Protocol drift: a connect response carries INIT_DATA, not a
                    // pool state transition. Surface it, but keep the reattach (the
                    // transition is tolerated, not fatal).
                    warn!(
                        message_type = ?MessageType::RunspacepoolState,
                        data_len,
                        "unexpected RunspacepoolState in ConnectResponse; ignoring"
                    );
                }
                other => {
                    // Be tolerant: unknown payloads must not kill the reattach.
                    let message_type = other.message_type();
                    warn!(
                        message_type = ?message_type,
                        message_type_value = message_type.value(),
                        data_len,
                        "ignoring unexpected PSRP message in ConnectResponse"
                    );
                }
            }
        }

        // A valid ConnectResponse must carry the negotiation (SessionCapability) and the
        // pool sizing (RunspacePoolInitData); without them the pool is not safely usable.
        // ApplicationPrivateData remains optional.
        if !saw_session_capability {
            return Err(crate::PwshCoreError::InvalidResponse(
                "ConnectResponse missing SessionCapability".into(),
            ));
        }
        if !saw_init_data {
            return Err(crate::PwshCoreError::InvalidResponse(
                "ConnectResponse missing RunspacePoolInitData".into(),
            ));
        }

        // The shell is attached and the pool is usable; the caller fires the
        // initial pool-stream Receive (mirrors the post-create handoff where
        // the pool-stream poll is already accounted for).
        runspace_pool.state = RunspacePoolState::Opened;
        runspace_pool.desired_stream_is_pooling = true;
        info!(
            runspace_pool_id = %runspace_pool.id,
            "runspace pool connected to existing shell"
        );

        Ok(runspace_pool)
    }
}
