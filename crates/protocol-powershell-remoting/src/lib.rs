pub mod cores;
pub mod fragment;
pub mod messages;

use core::error;
use std::str::Utf8Error;

pub use cores::*;
pub use fragment::*;
pub use messages::*;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PowerShellRemotingError {
    #[error("Invalid PowerShell remoting message: {0}")]
    InvalidMessage(String),

    #[error("PowerShell remoting error: {0}")]
    RemotingError(String),

    #[error("IO Error: {0}")]
    IoError(String),

    #[error("Serialization Error: {0}")]
    SerializationError(&'static str),

    #[error("PsFragment cannot be read as a valid XML message: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("Failed to parse XML: {0}")]
    XmlParseError(#[from] xml::XmlError),
}

impl From<std::io::Error> for PowerShellRemotingError {
    fn from(err: std::io::Error) -> Self {
        PowerShellRemotingError::IoError(err.to_string())
    }
}
