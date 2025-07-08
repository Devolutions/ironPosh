#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Invalid SOAP version: {0}")]
    InvalidSoapVersion(String),

    #[error("SOAP envelope is missing")]
    MissingSoapEnvelope,

    #[error("SOAP body is missing")]
    MissingSoapBody,

    #[error("SOAP header is missing")]
    MissingSoapHeader,

    #[error("XML parsing error: {0}")]
    XmlParsingError(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}
