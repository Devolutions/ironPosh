pub mod connector;
pub mod runspace;
pub mod runspace_pool;
pub mod pipeline;

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

    #[error("Something unlikely happened: {0}")]
    UnlikelyToHappen(&'static str),

    #[error("Protocol error: {0}")]
    PowerShellRemotingError(#[from] protocol_powershell_remoting::PowerShellRemotingError),

    #[error("XML parsing error: {0}")]
    XmlParsingError(#[from] xml::XmlError),

    #[error("Invalid response: {0}")]
    InvalidResponse(&'static str),
}
