use protocol_powershell_remoting::{PipelineHostCall, PsValue};
use uuid::Uuid;

pub mod raw_ui;
pub mod ui;

#[derive(Debug, Clone)]
pub enum HostCallType {
    Pipeline { id: Uuid },
    RunspacePool,
}

#[derive(Debug, Clone)]
pub struct HostCall {
    /// Type of the host call
    pub call_type: HostCallType,
    /// Unique identifier for this host call
    pub call_id: i64,
    /// The host method identifier (enum value)
    pub method_id: i32,
    /// String representation of the method name
    pub method_name: String,
    /// Parameters for the method call as a list of values
    pub parameters: Vec<PsValue>,
}

impl HostCall {
    pub fn new(
        call_type: HostCallType,
        call_id: i64,
        method_id: i32,
        method_name: String,
        parameters: Vec<PsValue>,
    ) -> Self {
        Self {
            call_type,
            call_id,
            method_id,
            method_name,
            parameters,
        }
    }
}

impl From<(&PipelineHostCall, HostCallType)> for HostCall {
    fn from((call, call_type): (&PipelineHostCall, HostCallType)) -> Self {
        let PipelineHostCall {
            call_id,
            method_id,
            method_name,
            parameters,
        } = call;

        Self {
            call_type,
            call_id: *call_id,
            method_id: *method_id,
            method_name: method_name.to_string(),
            parameters: parameters.to_vec(),
        }
    }
}

impl Into<PipelineHostCall> for HostCall {
    fn into(self) -> PipelineHostCall {
        PipelineHostCall {
            call_id: self.call_id,
            method_id: self.method_id,
            method_name: self.method_name,
            parameters: self.parameters,
        }
    }
}
