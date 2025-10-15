pub mod cores;
pub mod fragmentation;
pub mod messages;
pub mod ps_value;

use std::str::Utf8Error;

pub use cores::*;
pub use fragmentation::*;
pub use messages::*;
pub use ps_value::PsObjectWithType;

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
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
    XmlError(#[from] ironposh_xml::XmlError),

    #[error("Output formatting error: {0}")]
    OutputFormattingError(&'static str),
}

impl From<std::io::Error> for PowerShellRemotingError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}
