//! Typed PSRP message stream (RFC #12, layer L4).
//!
//! [`PsrpMessage`] is the typed view of a wire [`PowerShellRemotingMessage`]:
//! parsing happens once, at the boundary, instead of every consumer
//! re-running the "match `MessageType` → `parse_ps_message` → `try_from`"
//! dance. State machines can then `match` on a typed enum and stop knowing
//! that CLIXML exists.
//!
//! Message types that are not (yet) modeled as a dedicated typed struct — or
//! that legitimately carry a free-form value, such as the debug/verbose/warning
//! records — are preserved verbatim in [`PsrpMessage::Other`] /
//! [`PsrpMessage::DebugRecord`] etc., so no information is lost and the variant
//! set can grow incrementally.

use crate::ps_value::PsValue;
use crate::{
    ApplicationPrivateData, EncryptedSessionKey, ErrorRecord, InformationRecord, MessageType,
    PipelineHostCall, PipelineOutput, PipelineStateMessage, PowerShellRemotingError,
    PowerShellRemotingMessage, ProgressRecord, PublicKeyRequest, RunspacePoolHostCall,
    RunspacePoolInitData, RunspacePoolStateMessage, SessionCapability,
};

/// A wire message parsed into its typed payload.
///
/// Construct with [`PsrpMessage::parse`]. Unmodeled types land in
/// [`PsrpMessage::Other`].
#[derive(Debug, Clone)]
pub enum PsrpMessage {
    SessionCapability(SessionCapability),
    ApplicationPrivateData(ApplicationPrivateData),
    EncryptedSessionKey(EncryptedSessionKey),
    PublicKeyRequest(PublicKeyRequest),
    RunspacePoolState(RunspacePoolStateMessage),
    RunspacePoolInitData(RunspacePoolInitData),
    RunspacePoolHostCall(RunspacePoolHostCall),
    // Boxed: these records are large relative to the other variants.
    ProgressRecord(Box<ProgressRecord>),
    InformationRecord(Box<InformationRecord>),
    PipelineState(PipelineStateMessage),
    PipelineOutput(PipelineOutput),
    PipelineHostCall(PipelineHostCall),
    ErrorRecord(Box<ErrorRecord>),
    /// DEBUG_RECORD payload (a single string in practice; kept as the raw value).
    DebugRecord(PsValue),
    /// VERBOSE_RECORD payload.
    VerboseRecord(PsValue),
    /// WARNING_RECORD payload.
    WarningRecord(PsValue),
    /// Any message type without a dedicated typed variant, carrying the raw
    /// parsed value so consumers can still inspect or log it.
    Other {
        message_type: MessageType,
        value: PsValue,
    },
}

impl PsrpMessage {
    /// Parse a wire message into its typed form.
    ///
    /// Errors only when the payload is present but malformed for its declared
    /// type; unknown/unmodeled types are returned as [`PsrpMessage::Other`]
    /// rather than failing.
    pub fn parse(message: &PowerShellRemotingMessage) -> Result<Self, PowerShellRemotingError> {
        // PipelineOutput parses from the raw message (the value may be any
        // primitive or object, not just a property bag).
        if message.message_type == MessageType::PipelineOutput {
            return Ok(Self::PipelineOutput(PipelineOutput::try_from(message)?));
        }

        let value = message.parse_ps_message()?;
        Ok(match &message.message_type {
            MessageType::SessionCapability => {
                Self::SessionCapability(Self::expect_object(value)?.try_into()?)
            }
            MessageType::ApplicationPrivateData => {
                Self::ApplicationPrivateData(Self::expect_object(value)?.try_into()?)
            }
            MessageType::EncryptedSessionKey => {
                Self::EncryptedSessionKey(Self::expect_object(value)?.try_into()?)
            }
            MessageType::PublicKeyRequest => Self::PublicKeyRequest(value.try_into()?),
            MessageType::RunspacepoolState => {
                Self::RunspacePoolState(Self::expect_object(value)?.try_into()?)
            }
            MessageType::RunspacepoolInitData => {
                Self::RunspacePoolInitData(Self::expect_object(value)?.try_into()?)
            }
            MessageType::RunspacepoolHostCall => {
                Self::RunspacePoolHostCall(Self::expect_object(value)?.try_into()?)
            }
            MessageType::ProgressRecord => {
                Self::ProgressRecord(Box::new(Self::expect_object(value)?.try_into()?))
            }
            MessageType::InformationRecord => {
                Self::InformationRecord(Box::new(Self::expect_object(value)?.try_into()?))
            }
            MessageType::PipelineState => {
                Self::PipelineState(Self::expect_object(value)?.try_into()?)
            }
            MessageType::PipelineHostCall => {
                Self::PipelineHostCall(Self::expect_object(value)?.try_into()?)
            }
            MessageType::ErrorRecord => {
                Self::ErrorRecord(Box::new(Self::expect_object(value)?.try_into()?))
            }
            MessageType::DebugRecord => Self::DebugRecord(value),
            MessageType::VerboseRecord => Self::VerboseRecord(value),
            MessageType::WarningRecord => Self::WarningRecord(value),
            message_type => Self::Other {
                message_type: message_type.clone(),
                value,
            },
        })
    }

    /// The wire [`MessageType`] this variant corresponds to.
    pub fn message_type(&self) -> MessageType {
        match self {
            Self::SessionCapability(_) => MessageType::SessionCapability,
            Self::ApplicationPrivateData(_) => MessageType::ApplicationPrivateData,
            Self::EncryptedSessionKey(_) => MessageType::EncryptedSessionKey,
            Self::PublicKeyRequest(_) => MessageType::PublicKeyRequest,
            Self::RunspacePoolState(_) => MessageType::RunspacepoolState,
            Self::RunspacePoolInitData(_) => MessageType::RunspacepoolInitData,
            Self::RunspacePoolHostCall(_) => MessageType::RunspacepoolHostCall,
            Self::ProgressRecord(_) => MessageType::ProgressRecord,
            Self::InformationRecord(_) => MessageType::InformationRecord,
            Self::PipelineState(_) => MessageType::PipelineState,
            Self::PipelineOutput(_) => MessageType::PipelineOutput,
            Self::PipelineHostCall(_) => MessageType::PipelineHostCall,
            Self::ErrorRecord(_) => MessageType::ErrorRecord,
            Self::DebugRecord(_) => MessageType::DebugRecord,
            Self::VerboseRecord(_) => MessageType::VerboseRecord,
            Self::WarningRecord(_) => MessageType::WarningRecord,
            Self::Other { message_type, .. } => message_type.clone(),
        }
    }

    fn expect_object(
        value: PsValue,
    ) -> Result<crate::ps_value::ComplexObject, PowerShellRemotingError> {
        match value {
            PsValue::Object(obj) => Ok(obj),
            PsValue::Primitive(_) => Err(PowerShellRemotingError::InvalidMessage(
                "expected a ComplexObject payload".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Destination, PsObjectWithType};

    fn wire(message: &dyn PsObjectWithType) -> PowerShellRemotingMessage {
        PowerShellRemotingMessage::new(
            Destination::Client,
            message.message_type(),
            uuid::Uuid::nil(),
            None,
            &message.to_ps_object(),
        )
        .expect("build wire message")
    }

    #[test]
    fn parses_session_capability() {
        let cap = SessionCapability {
            protocol_version: "2.2".into(),
            ps_version: "2.0".into(),
            serialization_version: "1.1.0.1".into(),
            time_zone: None,
        };
        let msg = wire(&cap);
        match PsrpMessage::parse(&msg).expect("parse") {
            PsrpMessage::SessionCapability(parsed) => assert_eq!(parsed, cap),
            other => panic!("expected SessionCapability, got {other:?}"),
        }
    }

    #[test]
    fn parses_runspace_pool_init_data() {
        let init = RunspacePoolInitData {
            min_runspaces: 2,
            max_runspaces: 8,
        };
        let msg = wire(&init);
        match PsrpMessage::parse(&msg).expect("parse") {
            PsrpMessage::RunspacePoolInitData(parsed) => assert_eq!(parsed, init),
            other => panic!("expected RunspacePoolInitData, got {other:?}"),
        }
        assert_eq!(
            PsrpMessage::parse(&msg).unwrap().message_type(),
            MessageType::RunspacepoolInitData
        );
    }

    #[test]
    fn parses_pipeline_output_value() {
        let out = PipelineOutput {
            data: PsValue::from("hello"),
        };
        let msg = PowerShellRemotingMessage::new(
            Destination::Client,
            MessageType::PipelineOutput,
            uuid::Uuid::nil(),
            None,
            &out.data,
        )
        .unwrap();
        match PsrpMessage::parse(&msg).expect("parse") {
            PsrpMessage::PipelineOutput(parsed) => {
                assert_eq!(parsed.data.as_string().as_deref(), Some("hello"));
            }
            other => panic!("expected PipelineOutput, got {other:?}"),
        }
    }
}
