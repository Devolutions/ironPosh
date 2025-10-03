use serde::{Deserialize, Serialize};
use tracing::error;
use tsify::Tsify;
use wasm_bindgen::JsValue;

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("IO Error: {0}")]
    IOError(String),

    #[error("Internal Error: {0}")]
    IronPoshError(#[from] anyhow::Error),

    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("WebSocket Error: {0}")]
    WebSocket(String),

    #[error("internal powershell error: {0}")]
    PowerShellError(#[from] ironposh_psrp::PowerShellRemotingError),

    #[error("Serialization Error")]
    SerializationError(#[from] serde_wasm_bindgen::Error),

    #[error("URL Parse Error for target: {target}, source : {source}")]
    UrlParseError {
        source: url::ParseError,
        target: String,
    },
    
    #[error("Invalid Argument: {0}")]
    InvalidArgument(String),
}

unsafe impl Send for WasmError {}
unsafe impl Sync for WasmError {}

impl WasmError {
    pub fn name(&self) -> &str {
        match self {
            WasmError::IOError(_) => "IOError",
            WasmError::IronPoshError(_) => "IronPoshError",
            WasmError::Generic(_) => "GenericError",
            WasmError::WebSocket(_) => "WebSocketError",
            WasmError::PowerShellError(_) => "PowerShellError",
            WasmError::SerializationError(_) => "SerializationError",
            WasmError::UrlParseError { .. } => "UrlParseError",
            WasmError::InvalidArgument(_) => "InvalidArgument",
        }
    }
}

impl From<WasmError> for IronPoshError {
    fn from(value: WasmError) -> Self {
        error!(
            error_code = value.name(),
            error_message = %value,
            "converting WasmError to IronPoshError"
        );
        IronPoshError {
            code: value.name().to_string(),
            message: value.to_string(),
        }
    }
}

impl From<WasmError> for JsValue {
    fn from(value: WasmError) -> Self {
        error!(
            error_code = value.name(),
            error_message = %value,
            "converting WasmError to JsValue"
        );
        let api_error: IronPoshError = value.into();
        api_error.into()
    }
}

#[derive(Serialize, Deserialize, Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct IronPoshError {
    pub code: String,
    pub message: String,
}
