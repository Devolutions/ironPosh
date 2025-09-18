use uuid::Uuid;

// Strongly-typed data structures per MS-PSRP spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinates { pub x: i32, pub y: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size { pub width: i32, pub height: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferCell {
    pub character: char,
    pub foreground: i32, // Color enum underlying int
    pub background: i32, // Color enum underlying int
    pub flags: i32,      // BufferCellType, underlying int
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub virtual_key_code: i32,
    pub character: char,
    pub control_key_state: i32,
    pub key_down: bool,
}

/// PowerShell Host method calls (requests with parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum HostMethodParams {
    GetName,
    GetVersion,
    GetInstanceId,
    GetCurrentCulture,
    GetCurrentUICulture,
    SetShouldExit(i32), // exit_code
    EnterNestedPrompt,
    ExitNestedPrompt,
    NotifyBeginApplication,
    NotifyEndApplication,
    // IHostSupportsInteractiveSession methods
    PushRunspace,
    PopRunspace,
    GetIsRunspacePushed,
    GetRunspace,
}

/// PowerShell Host UI method calls (requests with parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum UIMethodParams {
    ReadLine,
    ReadLineAsSecureString,
    // Unified Write method that handles Write1/Write2/WriteLine1/WriteLine2/WriteLine3
    Write { text: String, fg: Option<i32>, bg: Option<i32>, newline: bool },
    WriteErrorLine(String),
    WriteDebugLine(String),
    // Keep PsValue for progress to preserve full ProgressRecord shape
    WriteProgress(ironposh_psrp::PsValue),
    WriteVerboseLine(String),
    WriteWarningLine(String),
    Prompt { caption: String, descriptions: ironposh_psrp::PsValue },
    PromptForChoice { caption: String, message: String, choices: ironposh_psrp::PsValue, default_choice: i32 },
    PromptForCredential { caption: String, message: String, user_name: String, target_name: String, extra: Option<ironposh_psrp::PsValue> },
    // IHostSupportsMultipleChoiceSelect method
    PromptForChoiceMultipleSelection,
}

/// PowerShell Host Raw UI method calls (requests with parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum RawUIMethodParams {
    GetForegroundColor,
    SetForegroundColor(i32),
    GetBackgroundColor,
    SetBackgroundColor(i32),
    GetCursorPosition,
    SetCursorPosition(i32, i32), // x, y
    GetWindowPosition,
    SetWindowPosition(i32, i32), // x, y
    GetCursorSize,
    SetCursorSize(i32),
    GetBufferSize,
    SetBufferSize(i32, i32), // width, height
    GetWindowSize,
    SetWindowSize(i32, i32), // width, height
    GetWindowTitle,
    SetWindowTitle(String),
    GetMaxWindowSize,
    GetMaxPhysicalWindowSize,
    GetKeyAvailable,
    ReadKey(i32), // options
    FlushInputBuffer,
    // Spec-compliant buffer operations
    SetBufferContentsArray { x: i32, y: i32, cells: Vec<BufferCell> },
    SetBufferContentsRect { cell: BufferCell, rect: Rectangle },
    GetBufferContents { rect: Rectangle },
    ScrollBufferContents { src: Rectangle, dest_x: i32, dest_y: i32, clip: Rectangle, fill: BufferCell },
}

/// Host call request containing the method and metadata
#[derive(Debug, Clone, PartialEq)]
pub enum HostCallMethodWithParams {
    HostMethod(HostMethodParams),
    UIMethod(UIMethodParams),
    RawUIMethod(RawUIMethodParams),
}

impl HostCallMethodWithParams {
    /// Submit the result and validate that the request and return types match
    /// Returns (method_result, method_exception)
    pub fn submit(
        self,
        result: HostCallMethodReturn,
    ) -> Result<
        (
            Option<ironposh_psrp::PsValue>,
            Option<ironposh_psrp::PsValue>,
        ),
        super::error::HostError,
    > {
        use ironposh_psrp::{PsPrimitiveValue, PsValue};

        // Validate that the request and return types match
        if !matches(&self, &result) {
            return Err(super::error::HostError::RequestReturnMismatch);
        }

        let (method_result, method_exception) = match result {
            HostCallMethodReturn::Error(error) => {
                // Convert error to PsValue exception
                let error_message = error.to_string();
                (
                    None,
                    Some(PsValue::Primitive(PsPrimitiveValue::Str(error_message))),
                )
            }
            HostCallMethodReturn::HostMethod(method_return) => {
                let result = match method_return {
                    HostMethodReturn::GetName(name) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(name)))
                    }
                    HostMethodReturn::GetVersion(version) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(version)))
                    }
                    HostMethodReturn::GetInstanceId(id) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(id.to_string())))
                    }
                    HostMethodReturn::GetCurrentCulture(culture) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(culture)))
                    }
                    HostMethodReturn::GetCurrentUICulture(culture) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(culture)))
                    }
                    HostMethodReturn::SetShouldExit
                    | HostMethodReturn::EnterNestedPrompt
                    | HostMethodReturn::ExitNestedPrompt
                    | HostMethodReturn::NotifyBeginApplication
                    | HostMethodReturn::NotifyEndApplication
                    | HostMethodReturn::PushRunspace
                    | HostMethodReturn::PopRunspace => None, // void returns
                    HostMethodReturn::GetIsRunspacePushed(pushed) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Bool(pushed)))
                    }
                    HostMethodReturn::GetRunspace(obj) => Some(obj), // Pass-through opaque object
                };
                (result, None)
            }
            HostCallMethodReturn::UIMethod(ui_return) => {
                let result = match ui_return {
                    UIMethodReturn::ReadLine(text) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(text)))
                    }
                    UIMethodReturn::ReadLineAsSecureString(secure_data) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Bytes(secure_data)))
                    }
                    UIMethodReturn::Prompt(dict) => {
                        // Convert BTreeMap to PsValue dictionary
                        let ps_dict: std::collections::BTreeMap<PsValue, PsValue> = dict
                            .into_iter()
                            .map(|(k, v)| (PsValue::Primitive(PsPrimitiveValue::Str(k)), v))
                            .collect();
                        
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: Some(ironposh_psrp::PsType::ps_primitive_dictionary()),
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Container(
                                ironposh_psrp::Container::Dictionary(ps_dict)
                            ),
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: std::collections::BTreeMap::new(),
                        }))
                    }
                    UIMethodReturn::PromptForChoice(choice) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::I32(choice)))
                    }
                    UIMethodReturn::PromptForCredential(username, password) => {
                        // Create a credential-like object
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "UserName".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "UserName".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Str(username)),
                            },
                        );
                        properties.insert(
                            "Password".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "Password".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Bytes(password)),
                            },
                        );
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: None,
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Standard,
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: properties,
                        }))
                    }
                    UIMethodReturn::PromptForChoiceMultipleSelection(choices) => {
                        // Convert Vec<i32> to PsValue array
                        let choice_values: Vec<PsValue> = choices
                            .into_iter()
                            .map(|c| PsValue::Primitive(PsPrimitiveValue::I32(c)))
                            .collect();
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: Some(ironposh_psrp::PsType::array_list()),
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Container(
                                ironposh_psrp::Container::List(choice_values)
                            ),
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: std::collections::BTreeMap::new(),
                        }))
                    }
                    UIMethodReturn::Write
                    | UIMethodReturn::WriteErrorLine
                    | UIMethodReturn::WriteDebugLine
                    | UIMethodReturn::WriteProgress
                    | UIMethodReturn::WriteVerboseLine
                    | UIMethodReturn::WriteWarningLine => None, // void returns
                };
                (result, None)
            }
            HostCallMethodReturn::RawUIMethod(raw_ui_return) => {
                let result = match raw_ui_return {
                    RawUIMethodReturn::GetForegroundColor(color) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::I32(color)))
                    }
                    RawUIMethodReturn::GetBackgroundColor(color) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::I32(color)))
                    }
                    RawUIMethodReturn::GetCursorPosition(x, y)
                    | RawUIMethodReturn::GetWindowPosition(x, y) => {
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "X".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "X".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(x)),
                            },
                        );
                        properties.insert(
                            "Y".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "Y".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(y)),
                            },
                        );
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: None,
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Standard,
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: properties,
                        }))
                    }
                    RawUIMethodReturn::GetCursorSize(size) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::I32(size)))
                    }
                    RawUIMethodReturn::GetBufferSize(width, height)
                    | RawUIMethodReturn::GetWindowSize(width, height)
                    | RawUIMethodReturn::GetMaxWindowSize(width, height)
                    | RawUIMethodReturn::GetMaxPhysicalWindowSize(width, height) => {
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "Width".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "Width".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(width)),
                            },
                        );
                        properties.insert(
                            "Height".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "Height".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(height)),
                            },
                        );
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: None,
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Standard,
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: properties,
                        }))
                    }
                    RawUIMethodReturn::GetWindowTitle(title) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Str(title)))
                    }
                    RawUIMethodReturn::GetKeyAvailable(available) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Bool(available)))
                    }
                    RawUIMethodReturn::ReadKey(virtual_key, character, control_state, key_down) => {
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "VirtualKeyCode".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "VirtualKeyCode".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(virtual_key)),
                            },
                        );
                        properties.insert(
                            "Character".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "Character".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Str(
                                    character.to_string(),
                                )),
                            },
                        );
                        properties.insert(
                            "ControlKeyState".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "ControlKeyState".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(control_state)),
                            },
                        );
                        properties.insert(
                            "KeyDown".to_string(),
                            ironposh_psrp::PsProperty {
                                name: "KeyDown".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Bool(key_down)),
                            },
                        );
                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: None,
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Standard,
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: properties,
                        }))
                    }
                    RawUIMethodReturn::GetBufferContents(contents) => {
                        // Convert Vec<BufferCell> to PsValue array
                        let cell_values: Vec<PsValue> = contents
                            .into_iter()
                            .map(|cell| {
                                let mut properties = std::collections::BTreeMap::new();
                                properties.insert(
                                    "Character".to_string(),
                                    ironposh_psrp::PsProperty {
                                        name: "Character".to_string(),
                                        value: PsValue::Primitive(PsPrimitiveValue::Str(cell.character.to_string())),
                                    },
                                );
                                properties.insert(
                                    "ForegroundColor".to_string(),
                                    ironposh_psrp::PsProperty {
                                        name: "ForegroundColor".to_string(),
                                        value: PsValue::Primitive(PsPrimitiveValue::I32(cell.foreground)),
                                    },
                                );
                                properties.insert(
                                    "BackgroundColor".to_string(),
                                    ironposh_psrp::PsProperty {
                                        name: "BackgroundColor".to_string(),
                                        value: PsValue::Primitive(PsPrimitiveValue::I32(cell.background)),
                                    },
                                );
                                properties.insert(
                                    "BufferCellType".to_string(),
                                    ironposh_psrp::PsProperty {
                                        name: "BufferCellType".to_string(),
                                        value: PsValue::Primitive(PsPrimitiveValue::I32(cell.flags)),
                                    },
                                );
                                PsValue::Object(ironposh_psrp::ComplexObject {
                                    type_def: None,
                                    to_string: None,
                                    content: ironposh_psrp::ComplexObjectContent::Standard,
                                    adapted_properties: std::collections::BTreeMap::new(),
                                    extended_properties: properties,
                                })
                            })
                            .collect();

                        Some(PsValue::Object(ironposh_psrp::ComplexObject {
                            type_def: Some(ironposh_psrp::PsType::array_list()),
                            to_string: None,
                            content: ironposh_psrp::ComplexObjectContent::Container(
                                ironposh_psrp::Container::List(cell_values)
                            ),
                            adapted_properties: std::collections::BTreeMap::new(),
                            extended_properties: std::collections::BTreeMap::new(),
                        }))
                    }
                    RawUIMethodReturn::SetForegroundColor
                    | RawUIMethodReturn::SetBackgroundColor
                    | RawUIMethodReturn::SetCursorPosition
                    | RawUIMethodReturn::SetWindowPosition
                    | RawUIMethodReturn::SetCursorSize
                    | RawUIMethodReturn::SetBufferSize
                    | RawUIMethodReturn::SetWindowSize
                    | RawUIMethodReturn::SetWindowTitle
                    | RawUIMethodReturn::FlushInputBuffer
                    | RawUIMethodReturn::SetBufferContentsArray
                    | RawUIMethodReturn::SetBufferContentsRect
                    | RawUIMethodReturn::ScrollBufferContents => None, // void returns
                };
                (result, None)
            }
        };

        Ok((method_result, method_exception))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{RemoteHostMethodId, should_send_host_response};

    #[test]
    fn test_spec_compliant_method_ids() {
        // Test that our method IDs match the MS-PSRP spec exactly
        assert_eq!(RemoteHostMethodId::GetName as i32, 1);
        assert_eq!(RemoteHostMethodId::GetCursorPosition as i32, 31); // This was broken before!
        assert_eq!(RemoteHostMethodId::SetCursorPosition as i32, 32); // This was broken before!
        assert_eq!(RemoteHostMethodId::GetForegroundColor as i32, 27);
        assert_eq!(RemoteHostMethodId::SetForegroundColor as i32, 28);
        assert_eq!(RemoteHostMethodId::PromptForChoiceMultipleSelection as i32, 56);
    }

    #[test]
    fn test_response_gating() {
        // Test that only methods with return values require responses
        assert!(should_send_host_response(RemoteHostMethodId::GetName));
        assert!(should_send_host_response(RemoteHostMethodId::ReadLine));
        assert!(should_send_host_response(RemoteHostMethodId::GetCursorPosition));
        
        // Test that void methods do NOT require responses
        assert!(!should_send_host_response(RemoteHostMethodId::SetShouldExit));
        assert!(!should_send_host_response(RemoteHostMethodId::SetCursorPosition));
        assert!(!should_send_host_response(RemoteHostMethodId::WriteProgress));
    }
}

/// PowerShell Host method responses (return values)
#[derive(Debug, Clone, PartialEq)]
pub enum HostMethodReturn {
    GetName(String),
    GetVersion(String), // Version string
    GetInstanceId(Uuid),
    GetCurrentCulture(String),   // CultureInfo
    GetCurrentUICulture(String), // CultureInfo
    SetShouldExit,               // void return
    EnterNestedPrompt,
    ExitNestedPrompt,
    NotifyBeginApplication,
    NotifyEndApplication,
    // IHostSupportsInteractiveSession methods
    PushRunspace,           // void return
    PopRunspace,            // void return
    GetIsRunspacePushed(bool), // Boolean return
    GetRunspace(ironposh_psrp::PsValue), // Object (opaque)
}

/// PowerShell Host UI method responses (return values)
#[derive(Debug, Clone, PartialEq)]
pub enum UIMethodReturn {
    ReadLine(String),
    ReadLineAsSecureString(Vec<u8>), // SecureString representation
    Write,                           // void return for unified Write method
    WriteErrorLine,                  // void return
    WriteDebugLine,                  // void return
    WriteProgress,                   // void return
    WriteVerboseLine,                // void return
    WriteWarningLine,                // void return
    Prompt(std::collections::BTreeMap<String, ironposh_psrp::PsValue>), // Dictionary<String,Any>
    PromptForChoice(i32),            // selected choice index
    PromptForCredential(String, Vec<u8>), // username, password (SecureString)
    PromptForChoiceMultipleSelection(Vec<i32>), // Collection<Int32>
}

/// PowerShell Host Raw UI method responses (return values)
#[derive(Debug, Clone, PartialEq)]
pub enum RawUIMethodReturn {
    GetForegroundColor(i32),
    SetForegroundColor, // void return
    GetBackgroundColor(i32),
    SetBackgroundColor,          // void return
    GetCursorPosition(i32, i32), // x, y (Coordinates)
    SetCursorPosition,           // void return
    GetWindowPosition(i32, i32), // x, y (Coordinates)
    SetWindowPosition,           // void return
    GetCursorSize(i32),          // percentage
    SetCursorSize,               // void return
    GetBufferSize(i32, i32),     // width, height (Size)
    SetBufferSize,               // void return
    GetWindowSize(i32, i32),     // width, height (Size)
    SetWindowSize,               // void return
    GetWindowTitle(String),      // title
    SetWindowTitle,              // void return
    GetMaxWindowSize(i32, i32),         // width, height (Size)
    GetMaxPhysicalWindowSize(i32, i32), // width, height (Size)
    GetKeyAvailable(bool),
    ReadKey(i32, char, i32, bool), // virtual_key_code, character, control_key_state, key_down (KeyInfo)
    FlushInputBuffer,             // void return
    SetBufferContentsArray,       // void return
    SetBufferContentsRect,        // void return
    GetBufferContents(Vec<BufferCell>), // BufferCell[] 
    ScrollBufferContents,         // void return
}

/// Complete host call response
#[derive(Debug, Clone, PartialEq)]
pub enum HostCallMethodReturn {
    HostMethod(HostMethodReturn),
    UIMethod(UIMethodReturn),
    RawUIMethod(RawUIMethodReturn),
    Error(super::error::HostError),
}

fn matches(params: &HostCallMethodWithParams, returns: &HostCallMethodReturn) -> bool {
    match (params, returns) {
        // Host method matches
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetName),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetName(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetVersion),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetVersion(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetInstanceId),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetInstanceId(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetCurrentCulture),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetCurrentCulture(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetCurrentUICulture),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetCurrentUICulture(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::SetShouldExit(_)),
            HostCallMethodReturn::HostMethod(HostMethodReturn::SetShouldExit),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::EnterNestedPrompt),
            HostCallMethodReturn::HostMethod(HostMethodReturn::EnterNestedPrompt),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::ExitNestedPrompt),
            HostCallMethodReturn::HostMethod(HostMethodReturn::ExitNestedPrompt),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::NotifyBeginApplication),
            HostCallMethodReturn::HostMethod(HostMethodReturn::NotifyBeginApplication),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::NotifyEndApplication),
            HostCallMethodReturn::HostMethod(HostMethodReturn::NotifyEndApplication),
        ) => true,
        // Interactive session methods
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::PushRunspace),
            HostCallMethodReturn::HostMethod(HostMethodReturn::PushRunspace),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::PopRunspace),
            HostCallMethodReturn::HostMethod(HostMethodReturn::PopRunspace),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetIsRunspacePushed),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetIsRunspacePushed(_)),
        ) => true,
        (
            HostCallMethodWithParams::HostMethod(HostMethodParams::GetRunspace),
            HostCallMethodReturn::HostMethod(HostMethodReturn::GetRunspace(_)),
        ) => true,

        // UI method matches
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::ReadLine),
            HostCallMethodReturn::UIMethod(UIMethodReturn::ReadLine(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::ReadLineAsSecureString),
            HostCallMethodReturn::UIMethod(UIMethodReturn::ReadLineAsSecureString(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::Write { .. }),
            HostCallMethodReturn::UIMethod(UIMethodReturn::Write),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteErrorLine(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteErrorLine),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteDebugLine(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteDebugLine),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteProgress(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteProgress),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteVerboseLine(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteVerboseLine),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteWarningLine(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteWarningLine),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::Prompt { .. }),
            HostCallMethodReturn::UIMethod(UIMethodReturn::Prompt(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::PromptForChoice { .. }),
            HostCallMethodReturn::UIMethod(UIMethodReturn::PromptForChoice(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::PromptForCredential { .. }),
            HostCallMethodReturn::UIMethod(UIMethodReturn::PromptForCredential(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::PromptForChoiceMultipleSelection),
            HostCallMethodReturn::UIMethod(UIMethodReturn::PromptForChoiceMultipleSelection(_)),
        ) => true,

        // RawUI method matches
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetForegroundColor),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetForegroundColor(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetForegroundColor(_)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetForegroundColor),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetBackgroundColor),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBackgroundColor(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetBackgroundColor(_)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetBackgroundColor),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetCursorPosition),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetCursorPosition(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetCursorPosition(_, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetCursorPosition),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetWindowPosition),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetWindowPosition(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetWindowPosition(_, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetWindowPosition),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetCursorSize),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetCursorSize(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetCursorSize(_)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetCursorSize),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetBufferSize),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferSize(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetBufferSize(_, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetBufferSize),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetWindowSize),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetWindowSize(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetWindowSize(_, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetWindowSize),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetWindowTitle),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetWindowTitle(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetWindowTitle(_)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetWindowTitle),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetMaxWindowSize),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetMaxWindowSize(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetMaxPhysicalWindowSize),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetMaxPhysicalWindowSize(_, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetKeyAvailable),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetKeyAvailable(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::ReadKey(_)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::ReadKey(_, _, _, _)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::FlushInputBuffer),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::FlushInputBuffer),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetBufferContentsArray { .. }),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetBufferContentsArray),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetBufferContentsRect { .. }),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetBufferContentsRect),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetBufferContents { .. }),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferContents(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::ScrollBufferContents { .. }),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::ScrollBufferContents),
        ) => true,

        // Error can match with any request
        (_, HostCallMethodReturn::Error(_)) => true,

        // No match
        _ => false,
    }
}
