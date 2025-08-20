use super::PsValue;
use crate::{MessageType, PowerShellRemotingError, PowerShellRemotingMessage, PsObjectWithType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineOutput {
    pub data: PsValue, // the actual output object (primitive or complex)
}

impl From<PsValue> for PipelineOutput {
    fn from(v: PsValue) -> Self { Self { data: v } }
}

impl PsObjectWithType for PipelineOutput {
    fn message_type(&self) -> MessageType {
        MessageType::PipelineOutput
    }

    // IMPORTANT: return the inner PsValue directly; no extra wrapper.
    fn to_ps_object(&self) -> PsValue {
        self.data.clone()
    }
}

impl TryFrom<&PowerShellRemotingMessage> for PipelineOutput {
    type Error = PowerShellRemotingError;

    fn try_from(msg: &PowerShellRemotingMessage) -> Result<Self, Self::Error> {
        if msg.message_type != MessageType::PipelineOutput {
            return Err(PowerShellRemotingError::InvalidMessage(
                "not a PipelineOutput message".into(),
            ));
        }
        Ok(PipelineOutput { data: msg.parse_ps_message()? })
    }
}