use protocol_powershell_remoting::{PipelineHostCall, PsValue};
use uuid::Uuid;

pub mod ps_host;
pub mod raw_ui;
pub mod ui;

// Re-export the traits for convenience
pub use ps_host::PSHost;
pub use raw_ui::PSHostRawUserInterface;
pub use ui::PSHostUserInterface;

/// Error type for host operations
#[derive(Debug, Clone)]
pub enum HostError {
    NotImplemented,
    InvalidParameters,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::NotImplemented => write!(f, "Operation not implemented"),
            HostError::InvalidParameters => write!(f, "Invalid parameters"),
            HostError::Cancelled => write!(f, "Operation cancelled"),
            HostError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Result type for host operations
pub type HostResult<T> = Result<T, HostError>;

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

impl From<HostCall> for PipelineHostCall {
    fn from(val: HostCall) -> Self {
        PipelineHostCall {
            call_id: val.call_id,
            method_id: val.method_id,
            method_name: val.method_name,
            parameters: val.parameters,
        }
    }
}
