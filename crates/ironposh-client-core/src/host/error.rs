/// Error type for host operations
#[derive(Debug, Clone, PartialEq, Eq)]
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
            Self::NotImplemented => write!(f, "Operation not implemented"),
            Self::InvalidParameters => write!(f, "Invalid parameters"),
            Self::RequestReturnMismatch => write!(f, "Request and return types do not match"),
            Self::Cancelled => write!(f, "Operation cancelled"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Result type for host operations
pub type HostResult<T> = Result<T, HostError>;
