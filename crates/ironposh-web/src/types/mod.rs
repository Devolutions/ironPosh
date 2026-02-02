mod hostcall;
mod hostcall_objects;
pub use hostcall::*;
pub use hostcall_objects::*;
use ironposh_async::SessionEvent;
use ironposh_psrp::{ErrorRecord, PipelineOutput};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::error::WasmError;

// =============================================================================
// HostCall handler typing helpers (TypeScript-only)
// =============================================================================

#[wasm_bindgen(typescript_custom_section)]
const HOSTCALL_HANDLER_TS_TYPES: &str = r#"
// Utility: sync or async value.
export type MaybePromise<T> = T | Promise<T>;

export type HostCallTag = JsHostCall extends infer U ? (U extends any ? keyof U : never) : never;
export type HostCallVariant<K extends HostCallTag> = Extract<JsHostCall, Record<K, unknown>>;

// Strongly-typed hostcall handler as overloads. This avoids downstream
// declaration merging and enforces correct return types per call variant.
export interface TypedHostCallHandler {
  // Host methods (1-10)
  (call: HostCallVariant<"GetName">): MaybePromise<string>;
  (call: HostCallVariant<"GetVersion">): MaybePromise<string>;
  (call: HostCallVariant<"GetInstanceId">): MaybePromise<string>;
  (call: HostCallVariant<"GetCurrentCulture">): MaybePromise<string>;
  (call: HostCallVariant<"GetCurrentUICulture">): MaybePromise<string>;
  (call: HostCallVariant<"SetShouldExit">): MaybePromise<void>;
  (call: HostCallVariant<"EnterNestedPrompt">): MaybePromise<void>;
  (call: HostCallVariant<"ExitNestedPrompt">): MaybePromise<void>;
  (call: HostCallVariant<"NotifyBeginApplication">): MaybePromise<void>;
  (call: HostCallVariant<"NotifyEndApplication">): MaybePromise<void>;

  // Input methods (11-14)
  (call: HostCallVariant<"ReadLine">): MaybePromise<string>;
  (call: HostCallVariant<"ReadLineAsSecureString">): MaybePromise<string | Uint8Array | number[]>;

  // Output methods (15-26)
  (call: HostCallVariant<"Write1">): MaybePromise<void>;
  (call: HostCallVariant<"Write2">): MaybePromise<void>;
  (call: HostCallVariant<"WriteLine1">): MaybePromise<void>;
  (call: HostCallVariant<"WriteLine2">): MaybePromise<void>;
  (call: HostCallVariant<"WriteLine3">): MaybePromise<void>;
  (call: HostCallVariant<"WriteErrorLine">): MaybePromise<void>;
  (call: HostCallVariant<"WriteDebugLine">): MaybePromise<void>;
  (call: HostCallVariant<"WriteProgress">): MaybePromise<void>;
  (call: HostCallVariant<"WriteVerboseLine">): MaybePromise<void>;
  (call: HostCallVariant<"WriteWarningLine">): MaybePromise<void>;
  (call: HostCallVariant<"Prompt">): MaybePromise<Record<string, JsPsValue>>;
  (call: HostCallVariant<"PromptForCredential1">): MaybePromise<JsPSCredential>;
  (call: HostCallVariant<"PromptForCredential2">): MaybePromise<JsPSCredential>;
  (call: HostCallVariant<"PromptForChoice">): MaybePromise<number>;

  // RawUI methods (27-51)
  (call: HostCallVariant<"GetForegroundColor">): MaybePromise<number>;
  (call: HostCallVariant<"SetForegroundColor">): MaybePromise<void>;
  (call: HostCallVariant<"GetBackgroundColor">): MaybePromise<number>;
  (call: HostCallVariant<"SetBackgroundColor">): MaybePromise<void>;
  (call: HostCallVariant<"GetCursorPosition">): MaybePromise<JsCoordinates>;
  (call: HostCallVariant<"SetCursorPosition">): MaybePromise<void>;
  (call: HostCallVariant<"GetWindowPosition">): MaybePromise<JsCoordinates>;
  (call: HostCallVariant<"SetWindowPosition">): MaybePromise<void>;
  (call: HostCallVariant<"GetCursorSize">): MaybePromise<number>;
  (call: HostCallVariant<"SetCursorSize">): MaybePromise<void>;
  (call: HostCallVariant<"GetBufferSize">): MaybePromise<JsSize>;
  (call: HostCallVariant<"SetBufferSize">): MaybePromise<void>;
  (call: HostCallVariant<"GetWindowSize">): MaybePromise<JsSize>;
  (call: HostCallVariant<"SetWindowSize">): MaybePromise<void>;
  (call: HostCallVariant<"GetWindowTitle">): MaybePromise<string>;
  (call: HostCallVariant<"SetWindowTitle">): MaybePromise<void>;
  (call: HostCallVariant<"GetMaxWindowSize">): MaybePromise<JsSize>;
  (call: HostCallVariant<"GetMaxPhysicalWindowSize">): MaybePromise<JsSize>;
  (call: HostCallVariant<"GetKeyAvailable">): MaybePromise<boolean>;
  (call: HostCallVariant<"ReadKey">): MaybePromise<JsKeyInfo>;
  (call: HostCallVariant<"FlushInputBuffer">): MaybePromise<void>;
  (call: HostCallVariant<"SetBufferContents1">): MaybePromise<void>;
  (call: HostCallVariant<"SetBufferContents2">): MaybePromise<void>;
  (call: HostCallVariant<"GetBufferContents">): MaybePromise<JsBufferCell[][]>;
  (call: HostCallVariant<"ScrollBufferContents">): MaybePromise<void>;

  // Interactive session methods (52-56)
  (call: HostCallVariant<"PushRunspace">): MaybePromise<void>;
  (call: HostCallVariant<"PopRunspace">): MaybePromise<void>;
  (call: HostCallVariant<"GetIsRunspacePushed">): MaybePromise<boolean>;
  (call: HostCallVariant<"GetRunspace">): MaybePromise<JsPsValue>;
  (call: HostCallVariant<"PromptForChoiceMultipleSelection">): MaybePromise<number[]>;
}
"#;

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
#[allow(clippy::large_enum_variant)]
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
    PipelineRecord {
        pipeline_id: String,
        record: WasmPsrpRecord,
    },
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmPsrpRecordMeta {
    pub message_type: String,
    pub message_type_value: u32,
    pub stream: String,
    pub command_id: Option<String>,
    pub data_len: usize,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmHostInformationMessage {
    pub message: String,
    pub foreground_color: Option<i32>,
    pub background_color: Option<i32>,
    pub no_new_line: bool,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WasmInformationMessageData {
    #[serde(rename = "string")]
    String { value: String },
    #[serde(rename = "hostInformationMessage")]
    HostInformationMessage { value: WasmHostInformationMessage },
    #[serde(rename = "object")]
    Object { value: JsPsValue },
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WasmPsrpRecord {
    #[serde(rename = "debug")]
    Debug {
        meta: WasmPsrpRecordMeta,
        message: String,
    },
    #[serde(rename = "verbose")]
    Verbose {
        meta: WasmPsrpRecordMeta,
        message: String,
    },
    #[serde(rename = "warning")]
    Warning {
        meta: WasmPsrpRecordMeta,
        message: String,
    },
    #[serde(rename = "information")]
    Information {
        meta: WasmPsrpRecordMeta,
        #[serde(rename = "messageData")]
        message_data: WasmInformationMessageData,
        source: Option<String>,
        #[serde(rename = "timeGenerated")]
        time_generated: Option<String>,
        tags: Option<Vec<String>>,
        user: Option<String>,
        computer: Option<String>,
        #[serde(rename = "processId")]
        process_id: Option<i32>,
    },
    #[serde(rename = "progress")]
    Progress {
        meta: WasmPsrpRecordMeta,
        activity: String,
        #[serde(rename = "activityId")]
        activity_id: i32,
        #[serde(rename = "statusDescription")]
        status_description: Option<String>,
        #[serde(rename = "currentOperation")]
        current_operation: Option<String>,
        #[serde(rename = "parentActivityId")]
        parent_activity_id: Option<i32>,
        #[serde(rename = "percentComplete")]
        percent_complete: i32,
        #[serde(rename = "secondsRemaining")]
        seconds_remaining: Option<i32>,
    },
    #[serde(rename = "unsupported")]
    Unsupported {
        meta: WasmPsrpRecordMeta,
        #[serde(rename = "dataPreview")]
        data_preview: String,
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
