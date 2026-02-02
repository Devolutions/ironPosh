mod hostcall;
pub use hostcall::*;
use ironposh_async::SessionEvent;
use ironposh_psrp::{ErrorRecord, PipelineOutput};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::error::WasmError;

// =============================================================================
// Security Warning Callback Type
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Callback for security warnings.
    /// Returns Promise<boolean>:
    /// - true = user accepts risk, continue connection
    /// - false = user rejects, abort connection
    #[wasm_bindgen(typescript_type = "(warnings: SecurityWarning[]) => Promise<boolean>")]
    pub type SecurityWarningCallback;
}

// =============================================================================
// Security Warning Types
// =============================================================================

/// Security warnings that can occur during connection setup
#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "PascalCase")]
pub enum SecurityWarning {
    /// Gateway channel uses WS instead of WSS (unencrypted WebSocket)
    GatewayChannelInsecure,
    /// Destination channel uses TCP without SSPI encryption
    DestinationChannelInsecure,
    /// Both channels are insecure - extremely dangerous!
    BothChannelsInsecure,
}

// =============================================================================
// Gateway Transport Mode
// =============================================================================

/// How the Gateway connects to the WinRM server
#[derive(Tsify, Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "PascalCase")]
pub enum GatewayTransport {
    /// Gateway uses TCP to WinRM (HTTP on port 5985)
    /// → SSPI message sealing is ENABLED (provides encryption)
    #[default]
    Tcp,
    /// Gateway uses TLS to WinRM (HTTPS on port 5986)
    /// → SSPI message sealing is DISABLED (TLS provides encryption)
    Tls,
}

// =============================================================================
// WinRM Destination
// =============================================================================

/// WinRM server destination configuration
#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WinRmDestination {
    /// WinRM server hostname or IP address
    pub host: String,
    /// WinRM server port (typically 5985 for HTTP, 5986 for HTTPS)
    pub port: u16,
    /// How the Gateway connects to WinRM (TCP or TLS)
    pub transport: GatewayTransport,
}

// =============================================================================
// Main Config
// =============================================================================

/// WASM-compatible WinRM connection configuration
#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmWinRmConfig {
    /// Authentication method
    #[serde(default)]
    pub auth: WasmAuthMethod,

    /// WinRM destination (host, port, transport mode)
    pub destination: WinRmDestination,

    /// Gateway WebSocket URL (ws:// or wss://)
    pub gateway_url: String,

    /// Gateway authentication token
    pub gateway_token: String,

    /// Username for WinRM authentication
    pub username: String,

    /// Password for WinRM authentication
    pub password: String,

    /// Optional domain for authentication
    pub domain: Option<String>,

    /// Optional locale
    pub locale: Option<String>,

    /// KDC proxy URL for Kerberos authentication
    pub kdc_proxy_url: Option<String>,

    /// Client computer name for Kerberos authentication
    pub client_computer_name: Option<String>,

    /// Terminal columns
    #[serde(default = "default_cols")]
    pub cols: u16,

    /// Terminal rows
    #[serde(default = "default_rows")]
    pub rows: u16,

    /// Force disable SSPI encryption even on TCP transport.
    /// WARNING: This makes the destination channel insecure!
    /// Only valid when transport is Tcp.
    #[serde(default)]
    pub force_insecure: Option<bool>,
}

fn default_cols() -> u16 {
    120
}

fn default_rows() -> u16 {
    30
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum WasmAuthMethod {
    #[default]
    Basic,
    Ntlm,
    Kerberos,
    Negotiate,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum WasmPowerShellEvent {
    PipelineCreated {
        pipeline_id: String,
    },
    PipelineFinished {
        pipeline_id: String,
    },
    PipelineOutput {
        pipeline_id: String,
        data: String,
    },
    PipelineError {
        pipeline_id: String,
        error: WasmErrorRecord,
    },
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

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmErrorRecord {
    pub message: String,
    pub command_name: Option<String>,
    pub was_thrown_from_throw_statement: bool,
    pub fully_qualified_error_id: Option<String>,
    pub target_object: Option<String>,
    pub error_category: Option<i32>,
    pub serialize_extended_info: bool,
    pub normal_formated_message: String,
}

impl From<&ErrorRecord> for WasmErrorRecord {
    fn from(value: &ErrorRecord) -> Self {
        Self {
            message: value.message.clone(),
            normal_formated_message: value.render_normal(),
            command_name: value.command_name.clone(),
            was_thrown_from_throw_statement: value.was_thrown_from_throw_statement,
            fully_qualified_error_id: value.fully_qualified_error_id.clone(),
            target_object: value.target_object.clone(),
            error_category: value.error_category.as_ref().map(|ec| ec.category),
            serialize_extended_info: value.serialize_extended_info,
        }
    }
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

// =============================================================================
// Tab completion (TabExpansion2 / CommandCompletion)
// =============================================================================

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmCompletionResult {
    pub completion_text: String,
    pub list_item_text: String,
    pub result_type: String,
    pub tool_tip: String,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WasmCommandCompletion {
    pub current_match_index: i32,
    pub replacement_index: i32,
    pub replacement_length: i32,
    pub completion_matches: Vec<WasmCompletionResult>,
}

impl From<&ironposh_psrp::CompletionResult> for WasmCompletionResult {
    fn from(value: &ironposh_psrp::CompletionResult) -> Self {
        Self {
            completion_text: value.completion_text.clone(),
            list_item_text: value.list_item_text.clone(),
            result_type: value.result_type.clone(),
            tool_tip: value.tool_tip.clone(),
        }
    }
}

impl From<&ironposh_psrp::CommandCompletion> for WasmCommandCompletion {
    fn from(value: &ironposh_psrp::CommandCompletion) -> Self {
        Self {
            current_match_index: value.current_match_index,
            replacement_index: value.replacement_index,
            replacement_length: value.replacement_length,
            completion_matches: value
                .completion_matches
                .iter()
                .map(WasmCompletionResult::from)
                .collect(),
        }
    }
}
