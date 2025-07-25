pub mod messages;
pub mod headers;

pub use headers::*;
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
}

impl From<std::io::Error> for PowerShellRemotingError {
    fn from(err: std::io::Error) -> Self {
        PowerShellRemotingError::IoError(err.to_string())
    }
}
