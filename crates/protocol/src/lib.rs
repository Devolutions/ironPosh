pub mod error;
pub mod http;
pub(crate) mod macros;
pub mod shell;
pub mod soap;
pub mod traits;
pub mod ws_addressing;
pub mod ws_management;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;
