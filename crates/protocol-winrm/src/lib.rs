pub mod cores;
pub mod error;
pub mod http;
pub(crate) mod macros;
pub mod rsp;
pub mod soap;
pub mod ws_addressing;
pub mod ws_management;
pub mod test_macro;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;
