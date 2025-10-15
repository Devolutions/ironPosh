mod hostcall;
pub use hostcall::*;
use ironposh_async::SessionEvent;
use ironposh_psrp::PipelineOutput;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::error::WasmError;

// WASM-compatible structs with tsify for TypeScript generation
#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmWinRmConfig {
    pub server: String,
    pub port: u16,
    pub use_https: bool,
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
    pub locale: Option<String>,
    pub gateway_url: String,
    pub gateway_token: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum WasmPowerShellEvent {
    PipelineCreated { pipeline_id: String },
    PipelineFinished { pipeline_id: String },
    PipelineOutput { pipeline_id: String, data: String },
    PipelineError { pipeline_id: String, error: String },
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmHostCallEvent {
    pub call_id: String,
    pub call_type: String,
    pub message: String,
    pub options: Option<Vec<String>>,
}

// Opaque
#[wasm_bindgen]
pub struct WasmPipelineOutput {
    output: PipelineOutput,
}

#[wasm_bindgen]
impl WasmPipelineOutput {
    #[wasm_bindgen]
    pub fn to_formatted_string(&self) -> Result<String, WasmError> {
        Ok(self.output.format_as_displyable_string()?)
    }

    #[wasm_bindgen]
    pub fn to_object(&self) -> Result<JsValue, WasmError> {
        let obj = serde_wasm_bindgen::to_value(&self.output)?;
        Ok(obj)
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum JsSessionEvent {
    ConnectionStarted,
    ConnectionEstablished,
    ActiveSessionStarted,
    ActiveSessionEnded,
    #[serde(rename = "error")]
    Error(String),
    Closed,
}

impl From<SessionEvent> for JsSessionEvent {
    fn from(value: SessionEvent) -> Self {
        match value {
            SessionEvent::ConnectionStarted => Self::ConnectionStarted,
            SessionEvent::ConnectionEstablished => Self::ConnectionEstablished,
            SessionEvent::ActiveSessionStarted => Self::ActiveSessionStarted,
            SessionEvent::ActiveSessionEnded => Self::ActiveSessionEnded,
            SessionEvent::Error(e) => Self::Error(e),
            SessionEvent::Closed => Self::Closed,
        }
    }
}
