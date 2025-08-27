/// Error type for host operations
#[derive(Debug, Clone, PartialEq)]
pub enum HostError {
    NotImplemented,
    InvalidParameters,
    RequestReturnMismatch,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::NotImplemented => write!(f, "Operation not implemented"),
            HostError::InvalidParameters => write!(f, "Invalid parameters"),
            HostError::RequestReturnMismatch => write!(f, "Request and return types do not match"),
            HostError::Cancelled => write!(f, "Operation cancelled"),
            HostError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Result type for host operations
pub type HostResult<T> = Result<T, HostError>;
