use std::borrow::Cow;

pub mod connector;
pub mod credentials;
pub mod host;
pub mod pipeline;
pub mod powershell;
pub mod runspace;
pub mod runspace_pool;

pub use connector::config::{Authentication, KerberosConfig, SspiAuthConfig};
pub use credentials::ClientAuthIdentity;

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
    PowerShellRemotingError(#[from] ironposh_psrp::PowerShellRemotingError),

    #[error("XML parsing error: {0}")]
    XmlParsingError(#[from] ironposh_xml::XmlError),

    #[error("Invalid response: {0}")]
    InvalidResponse(Cow<'static, str>),

    #[error("Host error: {0}")]
    HostError(#[from] crate::host::HostError),

    #[error("SSPI error: {0}")]
    SspiError(#[from] sspi::Error),

    #[error("SSPI username error: {0}")]
    UsernameError(&'static str),

    #[error("Authentication error: {0}")]
    Auth(&'static str),
}
