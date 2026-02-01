use ironposh_client_core::host::HostCall;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::hostcall_objects::{
    JsGetBufferContentsStructured, JsPromptForChoiceMultipleSelectionStructured,
    JsPromptForChoiceStructured, JsPromptStructured, JsPushRunspaceStructured,
    JsScrollBufferContentsStructured, JsSetBufferContentsStructured, JsWriteProgressStructured,
};

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
        params: JsWriteProgressStructured,
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
        params: JsPromptStructured,
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
        params: JsPromptForChoiceStructured,
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
        params: JsSetBufferContentsStructured,
        return_type: VoidReturnType,
    },
    SetBufferContents2 {
        params: JsSetBufferContentsStructured,
        return_type: VoidReturnType,
    },
    GetBufferContents {
        params: JsGetBufferContentsStructured,
        return_type: BufferCellArrayReturnType,
    },
    ScrollBufferContents {
        params: JsScrollBufferContentsStructured,
        return_type: VoidReturnType,
    },

    // Interactive session methods (52-56)
    PushRunspace {
        params: JsPushRunspaceStructured,
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
        params: JsPromptForChoiceMultipleSelectionStructured,
        return_type: I32ArrayReturnType,
    },
}

impl From<&HostCall> for JsHostCall {
    #[expect(clippy::too_many_lines)]
    fn from(host_call: &HostCall) -> Self {
        match host_call {
            // Host methods (1-10)
            HostCall::GetName { .. } => Self::GetName {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetVersion { .. } => Self::GetVersion {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetInstanceId { .. } => Self::GetInstanceId {
                params: (),
                return_type: UuidReturnType::new(),
            },
            HostCall::GetCurrentCulture { .. } => Self::GetCurrentCulture {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::GetCurrentUICulture { .. } => Self::GetCurrentUICulture {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::SetShouldExit { transport } => Self::SetShouldExit {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::EnterNestedPrompt { .. } => Self::EnterNestedPrompt {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::ExitNestedPrompt { .. } => Self::ExitNestedPrompt {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::NotifyBeginApplication { .. } => Self::NotifyBeginApplication {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::NotifyEndApplication { .. } => Self::NotifyEndApplication {
                params: (),
                return_type: VoidReturnType::new(),
            },

            // UI methods (11-26)
            HostCall::ReadLine { .. } => Self::ReadLine {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::ReadLineAsSecureString { .. } => Self::ReadLineAsSecureString {
                params: (),
                return_type: BytesReturnType::new(),
            },
            HostCall::Write1 { transport } => Self::Write1 {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::Write2 { transport } => Self::Write2 {
                params: transport.params.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine1 { .. } => Self::WriteLine1 {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine2 { transport } => Self::WriteLine2 {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteLine3 { transport } => Self::WriteLine3 {
                params: transport.params.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteErrorLine { transport } => Self::WriteErrorLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteDebugLine { transport } => Self::WriteDebugLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteProgress { transport } => Self::WriteProgress {
                params: JsWriteProgressStructured {
                    source_id: transport.params.0,
                    record: transport.params.1.clone().into(),
                },
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteVerboseLine { transport } => Self::WriteVerboseLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::WriteWarningLine { transport } => Self::WriteWarningLine {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::Prompt { transport } => Self::Prompt {
                params: JsPromptStructured {
                    caption: transport.params.0.clone(),
                    message: transport.params.1.clone(),
                    fields: transport.params.2.iter().cloned().map(Into::into).collect(),
                },
                return_type: HashMapReturnType::new(),
            },
            HostCall::PromptForCredential1 { transport } => Self::PromptForCredential1 {
                params: transport.params.clone(),
                return_type: CredentialReturnType::new(),
            },
            HostCall::PromptForCredential2 { transport } => Self::PromptForCredential2 {
                params: transport.params.clone(),
                return_type: CredentialReturnType::new(),
            },
            HostCall::PromptForChoice { transport } => Self::PromptForChoice {
                params: JsPromptForChoiceStructured {
                    caption: transport.params.0.clone(),
                    message: transport.params.1.clone(),
                    choices: transport.params.2.iter().cloned().map(Into::into).collect(),
                    default_choice: transport.params.3,
                },
                return_type: I32ReturnType::new(),
            },

            // RawUI methods (27-51)
            HostCall::GetForegroundColor { .. } => Self::GetForegroundColor {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetForegroundColor { transport } => Self::SetForegroundColor {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBackgroundColor { .. } => Self::GetBackgroundColor {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetBackgroundColor { transport } => Self::SetBackgroundColor {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetCursorPosition { .. } => Self::GetCursorPosition {
                params: (),
                return_type: CoordinatesReturnType::new(),
            },
            HostCall::SetCursorPosition { transport } => Self::SetCursorPosition {
                params: (transport.params.0.x, transport.params.0.y),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowPosition { .. } => Self::GetWindowPosition {
                params: (),
                return_type: CoordinatesReturnType::new(),
            },
            HostCall::SetWindowPosition { transport } => Self::SetWindowPosition {
                params: (transport.params.0.x, transport.params.0.y),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetCursorSize { .. } => Self::GetCursorSize {
                params: (),
                return_type: I32ReturnType::new(),
            },
            HostCall::SetCursorSize { transport } => Self::SetCursorSize {
                params: transport.params.0,
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBufferSize { .. } => Self::GetBufferSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::SetBufferSize { transport } => Self::SetBufferSize {
                params: (transport.params.0.width, transport.params.0.height),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowSize { .. } => Self::GetWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::SetWindowSize { transport } => Self::SetWindowSize {
                params: (transport.params.0.width, transport.params.0.height),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetWindowTitle { .. } => Self::GetWindowTitle {
                params: (),
                return_type: StringReturnType::new(),
            },
            HostCall::SetWindowTitle { transport } => Self::SetWindowTitle {
                params: transport.params.0.clone(),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetMaxWindowSize { .. } => Self::GetMaxWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::GetMaxPhysicalWindowSize { .. } => Self::GetMaxPhysicalWindowSize {
                params: (),
                return_type: SizeReturnType::new(),
            },
            HostCall::GetKeyAvailable { .. } => Self::GetKeyAvailable {
                params: (),
                return_type: BoolReturnType::new(),
            },
            HostCall::ReadKey { transport } => Self::ReadKey {
                params: transport.params.0,
                return_type: KeyInfoReturnType::new(),
            },
            HostCall::FlushInputBuffer { .. } => Self::FlushInputBuffer {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::SetBufferContents1 { transport } => Self::SetBufferContents1 {
                params: JsSetBufferContentsStructured {
                    rect: transport.params.0.into(),
                    cell: transport.params.1.clone().into(),
                },
                return_type: VoidReturnType::new(),
            },
            HostCall::SetBufferContents2 { transport } => Self::SetBufferContents2 {
                params: JsSetBufferContentsStructured {
                    rect: transport.params.0.into(),
                    cell: transport.params.1.clone().into(),
                },
                return_type: VoidReturnType::new(),
            },
            HostCall::GetBufferContents { transport } => Self::GetBufferContents {
                params: JsGetBufferContentsStructured {
                    rect: transport.params.0.into(),
                },
                return_type: BufferCellArrayReturnType::new(),
            },
            HostCall::ScrollBufferContents { transport } => Self::ScrollBufferContents {
                params: JsScrollBufferContentsStructured {
                    source: transport.params.0.into(),
                    destination: transport.params.1.into(),
                    clip: transport.params.2.into(),
                    fill: transport.params.3.clone().into(),
                },
                return_type: VoidReturnType::new(),
            },

            // Interactive session methods (52-56)
            HostCall::PushRunspace { transport } => Self::PushRunspace {
                params: JsPushRunspaceStructured {
                    runspace: transport.params.0.clone().into(),
                },
                return_type: VoidReturnType::new(),
            },
            HostCall::PopRunspace { .. } => Self::PopRunspace {
                params: (),
                return_type: VoidReturnType::new(),
            },
            HostCall::GetIsRunspacePushed { .. } => Self::GetIsRunspacePushed {
                params: (),
                return_type: BoolReturnType::new(),
            },
            HostCall::GetRunspace { .. } => Self::GetRunspace {
                params: (),
                return_type: PsValueReturnType::new(),
            },
            HostCall::PromptForChoiceMultipleSelection { transport } => {
                Self::PromptForChoiceMultipleSelection {
                    params: JsPromptForChoiceMultipleSelectionStructured {
                        caption: transport.params.0.clone(),
                        message: transport.params.1.clone(),
                        choices: transport.params.2.iter().cloned().map(Into::into).collect(),
                        default_choices: transport.params.3.clone(),
                    },
                    return_type: I32ArrayReturnType::new(),
                }
            }
        }
    }
}
