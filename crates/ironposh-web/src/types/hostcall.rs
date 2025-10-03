use ironposh_client_core::host::HostCall;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct StringReturnType {
    name: String,
}

impl StringReturnType {
    fn new() -> Self {
        Self {
            name: "string".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct UuidReturnType {
    name: String,
}

impl UuidReturnType {
    fn new() -> Self {
        Self {
            name: "uuid".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct VoidReturnType {
    name: String,
}

impl VoidReturnType {
    fn new() -> Self {
        Self {
            name: "void".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct BytesReturnType {
    name: String,
}

impl BytesReturnType {
    fn new() -> Self {
        Self {
            name: "bytes".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct I32ReturnType {
    name: String,
}

impl I32ReturnType {
    fn new() -> Self {
        Self {
            name: "i32".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct BoolReturnType {
    name: String,
}

impl BoolReturnType {
    fn new() -> Self {
        Self {
            name: "bool".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct HashMapReturnType {
    name: String,
}

impl HashMapReturnType {
    fn new() -> Self {
        Self {
            name: "hashmap".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CredentialReturnType {
    name: String,
}

impl CredentialReturnType {
    fn new() -> Self {
        Self {
            name: "credential".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CoordinatesReturnType {
    name: String,
}

impl CoordinatesReturnType {
    fn new() -> Self {
        Self {
            name: "coordinates".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SizeReturnType {
    name: String,
}

impl SizeReturnType {
    fn new() -> Self {
        Self {
            name: "size".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct KeyInfoReturnType {
    name: String,
}

impl KeyInfoReturnType {
    fn new() -> Self {
        Self {
            name: "keyinfo".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct BufferCellArrayReturnType {
    name: String,
}

impl BufferCellArrayReturnType {
    fn new() -> Self {
        Self {
            name: "buffercellarray".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct PsValueReturnType {
    name: String,
}

impl PsValueReturnType {
    fn new() -> Self {
        Self {
            name: "psvalue".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct I32ArrayReturnType {
    name: String,
}

impl I32ArrayReturnType {
    fn new() -> Self {
        Self {
            name: "i32array".to_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum JsHostCall {
    // Host methods (1-10)
    GetName {
        params: (),
        return_type: StringReturnType,
    },
    GetVersion {
        params: (),
        return_type: StringReturnType,
    },
    GetInstanceId {
        params: (),
        return_type: UuidReturnType,
    },
    GetCurrentCulture {
        params: (),
        return_type: StringReturnType,
    },
    GetCurrentUICulture {
        params: (),
        return_type: StringReturnType,
    },
    SetShouldExit {
        params: i32,
        return_type: VoidReturnType,
    },
    EnterNestedPrompt {
        params: (),
        return_type: VoidReturnType,
    },
    ExitNestedPrompt {
        params: (),
        return_type: VoidReturnType,
    },
    NotifyBeginApplication {
        params: (),
        return_type: VoidReturnType,
    },
    NotifyEndApplication {
        params: (),
        return_type: VoidReturnType,
    },

    // UI methods (11-26)
    ReadLine {
        params: (),
        return_type: StringReturnType,
    },
    ReadLineAsSecureString {
        params: (),
        return_type: BytesReturnType,
    },
    Write1 {
        params: String,
        return_type: VoidReturnType,
    },
    Write2 {
        params: (i32, i32, String),
        return_type: VoidReturnType,
    },
    WriteLine1 {
        params: (),
        return_type: VoidReturnType,
    },
    WriteLine2 {
        params: String,
        return_type: VoidReturnType,
    },
    WriteLine3 {
        params: (i32, i32, String),
        return_type: VoidReturnType,
    },
    WriteErrorLine {
        params: String,
        return_type: VoidReturnType,
    },
    WriteDebugLine {
        params: String,
        return_type: VoidReturnType,
    },
    WriteProgress {
        params: (i64, String),
        return_type: VoidReturnType,
    },
    WriteVerboseLine {
        params: String,
        return_type: VoidReturnType,
    },
    WriteWarningLine {
        params: String,
        return_type: VoidReturnType,
    },
    Prompt {
        params: (String, String, Vec<String>),
        return_type: HashMapReturnType,
    },
    PromptForCredential1 {
        params: (String, String, String, String),
        return_type: CredentialReturnType,
    },
    PromptForCredential2 {
        params: (String, String, String, String, i32, i32),
        return_type: CredentialReturnType,
    },
    PromptForChoice {
        params: (String, String, Vec<String>, i32),
        return_type: I32ReturnType,
    },

    // RawUI methods (27-51)
    GetForegroundColor {
        params: (),
        return_type: I32ReturnType,
    },
    SetForegroundColor {
        params: i32,
        return_type: VoidReturnType,
    },
    GetBackgroundColor {
        params: (),
        return_type: I32ReturnType,
    },
    SetBackgroundColor {
        params: i32,
        return_type: VoidReturnType,
    },
    GetCursorPosition {
        params: (),
        return_type: CoordinatesReturnType,
    },
    SetCursorPosition {
        params: (i32, i32),
        return_type: VoidReturnType,
    },
    GetWindowPosition {
        params: (),
        return_type: CoordinatesReturnType,
    },
    SetWindowPosition {
        params: (i32, i32),
        return_type: VoidReturnType,
    },
    GetCursorSize {
        params: (),
        return_type: I32ReturnType,
    },
    SetCursorSize {
        params: i32,
        return_type: VoidReturnType,
    },
    GetBufferSize {
        params: (),
        return_type: SizeReturnType,
    },
    SetBufferSize {
        params: (i32, i32),
        return_type: VoidReturnType,
    },
    GetWindowSize {
        params: (),
        return_type: SizeReturnType,
    },
    SetWindowSize {
        params: (i32, i32),
        return_type: VoidReturnType,
    },
    GetWindowTitle {
        params: (),
        return_type: StringReturnType,
    },
    SetWindowTitle {
        params: String,
        return_type: VoidReturnType,
    },
    GetMaxWindowSize {
        params: (),
        return_type: SizeReturnType,
    },
    GetMaxPhysicalWindowSize {
        params: (),
        return_type: SizeReturnType,
    },
    GetKeyAvailable {
        params: (),
        return_type: BoolReturnType,
    },
    ReadKey {
        params: i32,
        return_type: KeyInfoReturnType,
    },
    FlushInputBuffer {
        params: (),
        return_type: VoidReturnType,
    },
    SetBufferContents1 {
        params: String,
        return_type: VoidReturnType,
    },
    SetBufferContents2 {
        params: String,
        return_type: VoidReturnType,
    },
    GetBufferContents {
        params: String,
        return_type: BufferCellArrayReturnType,
    },
    ScrollBufferContents {
        params: String,
        return_type: VoidReturnType,
    },

    // Interactive session methods (52-56)
    PushRunspace {
        params: String,
        return_type: VoidReturnType,
    },
    PopRunspace {
        params: (),
        return_type: VoidReturnType,
    },
    GetIsRunspacePushed {
        params: (),
        return_type: BoolReturnType,
    },
    GetRunspace {
        params: (),
        return_type: PsValueReturnType,
    },
    PromptForChoiceMultipleSelection {
        params: (String, String, Vec<String>, Vec<i32>),
        return_type: I32ArrayReturnType,
    },
}

impl From<&HostCall> for JsHostCall {
    fn from(host_call: &HostCall) -> Self {
        match host_call {
            // Host methods (1-10)
            HostCall::GetName { .. } => JsHostCall::GetName {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetVersion { .. } => JsHostCall::GetVersion {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetInstanceId { .. } => JsHostCall::GetInstanceId {
                params: (),
                return_type: UuidReturnType::new(),
            },
            HostCall::GetCurrentCulture { .. } => JsHostCall::GetCurrentCulture {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetCurrentUICulture { .. } => JsHostCall::GetCurrentUICulture {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::SetShouldExit { transport } => JsHostCall::SetShouldExit {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::EnterNestedPrompt { .. } => JsHostCall::EnterNestedPrompt {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::ExitNestedPrompt { .. } => JsHostCall::ExitNestedPrompt {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::NotifyBeginApplication { .. } => JsHostCall::NotifyBeginApplication {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::NotifyEndApplication { .. } => JsHostCall::NotifyEndApplication {
                params: (),
                return_type: VoidReturnType::new(),
            },

            // UI methods (11-26)
            HostCall::ReadLine { .. } => JsHostCall::ReadLine {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::ReadLineAsSecureString { .. } => JsHostCall::ReadLineAsSecureString {
                params: (),
                return_type: BytesReturnType::new(),
            },
            HostCall::Write1 { transport } => JsHostCall::Write1 {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::Write2 { transport } => JsHostCall::Write2 {
                params: transport.params.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine1 { .. } => JsHostCall::WriteLine1 {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine2 { transport } => JsHostCall::WriteLine2 {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine3 { transport } => JsHostCall::WriteLine3 {
                params: transport.params.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteErrorLine { transport } => JsHostCall::WriteErrorLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteDebugLine { transport } => JsHostCall::WriteDebugLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteProgress { transport } => JsHostCall::WriteProgress {
                params: (transport.params.0, format!("{:?}", transport.params.1)),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteVerboseLine { transport } => JsHostCall::WriteVerboseLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteWarningLine { transport } => JsHostCall::WriteWarningLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::Prompt { transport } => JsHostCall::Prompt {
                params: (
                    transport.params.0.clone(),
                    transport.params.1.clone(),
                    transport
                        .params
                        .2
                        .iter()
                        .map(|f| format!("{:?}", f))
                        .collect(),
                ),
                return_type: HashMapReturnType::new(),
            },
            HostCall::PromptForCredential1 { transport } => JsHostCall::PromptForCredential1 {
                params: transport.params.clone(),
                return_type: CredentialReturnType::new(),
            },
            HostCall::PromptForCredential2 { transport } => JsHostCall::PromptForCredential2 {
                params: transport.params.clone(),
                return_type: CredentialReturnType::new(),
            },
            HostCall::PromptForChoice { transport } => JsHostCall::PromptForChoice {
                params: (
                    transport.params.0.clone(),
                    transport.params.1.clone(),
                    transport
                        .params
                        .2
                        .iter()
                        .map(|c| format!("{:?}", c))
                        .collect(),
                    transport.params.3,
                ),
                return_type: I32ReturnType::new(),
            },

            // RawUI methods (27-51)
            HostCall::GetForegroundColor { .. } => JsHostCall::GetForegroundColor {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetForegroundColor { transport } => JsHostCall::SetForegroundColor {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBackgroundColor { .. } => JsHostCall::GetBackgroundColor {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetBackgroundColor { transport } => JsHostCall::SetBackgroundColor {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetCursorPosition { .. } => JsHostCall::GetCursorPosition {
                params: (),
                return_type: CoordinatesReturnType::new(),
            },
            HostCall::SetCursorPosition { transport } => JsHostCall::SetCursorPosition {
                params: (transport.params.0.x, transport.params.0.y),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowPosition { .. } => JsHostCall::GetWindowPosition {
                params: (),
                return_type: CoordinatesReturnType::new(),
            },
            HostCall::SetWindowPosition { transport } => JsHostCall::SetWindowPosition {
                params: (transport.params.0.x, transport.params.0.y),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetCursorSize { .. } => JsHostCall::GetCursorSize {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetCursorSize { transport } => JsHostCall::SetCursorSize {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBufferSize { .. } => JsHostCall::GetBufferSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::SetBufferSize { transport } => JsHostCall::SetBufferSize {
                params: (transport.params.0.width, transport.params.0.height),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowSize { .. } => JsHostCall::GetWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::SetWindowSize { transport } => JsHostCall::SetWindowSize {
                params: (transport.params.0.width, transport.params.0.height),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowTitle { .. } => JsHostCall::GetWindowTitle {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::SetWindowTitle { transport } => JsHostCall::SetWindowTitle {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetMaxWindowSize { .. } => JsHostCall::GetMaxWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::GetMaxPhysicalWindowSize { .. } => JsHostCall::GetMaxPhysicalWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::GetKeyAvailable { .. } => JsHostCall::GetKeyAvailable {
                params: (),
                return_type: BoolReturnType::new(),
            },
            HostCall::ReadKey { transport } => JsHostCall::ReadKey {
                params: transport.params.0,
                return_type: KeyInfoReturnType::new(),
            },
            HostCall::FlushInputBuffer { .. } => JsHostCall::FlushInputBuffer {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::SetBufferContents1 { transport } => JsHostCall::SetBufferContents1 {
                params: format!("{:?}", transport.params),
                return_type: VoidReturnType::new(),
            },
            HostCall::SetBufferContents2 { transport } => JsHostCall::SetBufferContents2 {
                params: format!("{:?}", transport.params),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBufferContents { transport } => JsHostCall::GetBufferContents {
                params: format!("{:?}", transport.params),
                return_type: BufferCellArrayReturnType::new(),
            },
            HostCall::ScrollBufferContents { transport } => JsHostCall::ScrollBufferContents {
                params: format!("{:?}", transport.params),
                return_type: VoidReturnType::new(),
            },

            // Interactive session methods (52-56)
            HostCall::PushRunspace { transport } => JsHostCall::PushRunspace {
                params: format!("{:?}", transport.params),
                return_type: VoidReturnType::new(),
            },
            HostCall::PopRunspace { .. } => JsHostCall::PopRunspace {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetIsRunspacePushed { .. } => JsHostCall::GetIsRunspacePushed {
                params: (),
                return_type: BoolReturnType::new(),
            },
            HostCall::GetRunspace { .. } => JsHostCall::GetRunspace {
                params: (),
                return_type: PsValueReturnType::new(),
            },
            HostCall::PromptForChoiceMultipleSelection { transport } => {
                JsHostCall::PromptForChoiceMultipleSelection {
                    params: (
                        transport.params.0.clone(),
                        transport.params.1.clone(),
                        transport
                            .params
                            .2
                            .iter()
                            .map(|c| format!("{:?}", c))
                            .collect(),
                        transport.params.3.clone(),
                    ),
                    return_type: I32ArrayReturnType::new(),
                }
            }
        }
    }
}
