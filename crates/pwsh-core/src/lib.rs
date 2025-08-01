pub mod powershell;
pub mod runspace;

#[derive(Debug, thiserror::Error)]
pub enum PwshCoreError {
    #[error("Connector error: {0}")]
    ConnectorError(String),

    #[error("Runspace error: {0}")]
    RunspaceError(String),

    #[error("Hyper error: {0}")]
    IOError(std::io::Error),

    #[error("Hyper error: {0}")]
    HyperError(#[from] hyper::http::Error),

    #[error("Invalid state: {0}")]
    InvalidState(&'static str),
}
