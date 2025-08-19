use uuid::Uuid;

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
}

/// PowerShell Host UI method calls (requests with parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum UIMethodParams {
    ReadLine,
    ReadLineAsSecureString,
    Write(String),                                       // value
    WriteLine(String),                                   // value
    WriteErrorLine(String),                              // value
    WriteDebugLine(String),                              // value
    WriteProgress(i64, String),                          // source_id, record
    WriteVerboseLine(String),                            // value
    WriteWarningLine(String),                            // value
    Prompt(String, Vec<String>),                         // caption, descriptions
    PromptForChoice(String, String, Vec<String>, i32), // caption, message, choices, default_choice
    PromptForCredential(String, String, String, String), // caption, message, user_name, target_name
}

/// PowerShell Host Raw UI method calls (requests with parameters)
#[derive(Debug, Clone, PartialEq)]
pub enum RawUIMethodParams {
    GetForegroundColor,
    SetForegroundColor(i32), // color
    GetBackgroundColor,
    SetBackgroundColor(i32), // color
    GetCursorPosition,
    SetCursorPosition(i32, i32), // x, y
    GetWindowPosition,
    SetWindowPosition(i32, i32), // x, y
    GetCursorSize,
    SetCursorSize(i32), // percentage
    GetBufferSize,
    SetBufferSize(i32, i32), // width, height
    GetWindowSize,
    SetWindowSize(i32, i32), // width, height
    GetMaxWindowSize,
    GetMaxPhysicalWindowSize,
    GetKeyAvailable,
    ReadKey(i32), // options
    FlushInputBuffer,
    SetBufferContents(i32, i32, String),   // x, y, contents
    GetBufferContents(i32, i32, i32, i32), // x, y, width, height
    ScrollBufferContents(i32, i32, i32, i32, i32, i32, String), // source rectangle, destination, clip, fill
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
            Option<protocol_powershell_remoting::PsValue>,
            Option<protocol_powershell_remoting::PsValue>,
        ),
        super::error::HostError,
    > {
        use protocol_powershell_remoting::{PsPrimitiveValue, PsValue};

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
                    | HostMethodReturn::NotifyEndApplication => None, // void returns
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
                    UIMethodReturn::Prompt(values) => Some(PsValue::from_string_array(values)),
                    UIMethodReturn::PromptForChoice(choice) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::I32(choice)))
                    }
                    UIMethodReturn::PromptForCredential(username, password) => {
                        // Create a credential-like object
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "UserName".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "UserName".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Str(username)),
                            },
                        );
                        properties.insert(
                            "Password".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "Password".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Bytes(password)),
                            },
                        );
                        Some(PsValue::Object(
                            protocol_powershell_remoting::ComplexObject {
                                type_def: None,
                                to_string: None,
                                content:
                                    protocol_powershell_remoting::ComplexObjectContent::Standard,
                                adapted_properties: std::collections::BTreeMap::new(),
                                extended_properties: properties,
                            },
                        ))
                    }
                    UIMethodReturn::Write
                    | UIMethodReturn::WriteLine
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
                            protocol_powershell_remoting::PsProperty {
                                name: "X".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(x)),
                            },
                        );
                        properties.insert(
                            "Y".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "Y".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(y)),
                            },
                        );
                        Some(PsValue::Object(
                            protocol_powershell_remoting::ComplexObject {
                                type_def: None,
                                to_string: None,
                                content:
                                    protocol_powershell_remoting::ComplexObjectContent::Standard,
                                adapted_properties: std::collections::BTreeMap::new(),
                                extended_properties: properties,
                            },
                        ))
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
                            protocol_powershell_remoting::PsProperty {
                                name: "Width".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(width)),
                            },
                        );
                        properties.insert(
                            "Height".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "Height".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(height)),
                            },
                        );
                        Some(PsValue::Object(
                            protocol_powershell_remoting::ComplexObject {
                                type_def: None,
                                to_string: None,
                                content:
                                    protocol_powershell_remoting::ComplexObjectContent::Standard,
                                adapted_properties: std::collections::BTreeMap::new(),
                                extended_properties: properties,
                            },
                        ))
                    }
                    RawUIMethodReturn::GetKeyAvailable(available) => {
                        Some(PsValue::Primitive(PsPrimitiveValue::Bool(available)))
                    }
                    RawUIMethodReturn::ReadKey(virtual_key, character, control_state, key_down) => {
                        let mut properties = std::collections::BTreeMap::new();
                        properties.insert(
                            "VirtualKeyCode".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "VirtualKeyCode".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(virtual_key)),
                            },
                        );
                        properties.insert(
                            "Character".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "Character".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Str(
                                    character.to_string(),
                                )),
                            },
                        );
                        properties.insert(
                            "ControlKeyState".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "ControlKeyState".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(control_state)),
                            },
                        );
                        properties.insert(
                            "KeyDown".to_string(),
                            protocol_powershell_remoting::PsProperty {
                                name: "KeyDown".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::I32(key_down)),
                            },
                        );
                        Some(PsValue::Object(
                            protocol_powershell_remoting::ComplexObject {
                                type_def: None,
                                to_string: None,
                                content:
                                    protocol_powershell_remoting::ComplexObjectContent::Standard,
                                adapted_properties: std::collections::BTreeMap::new(),
                                extended_properties: properties,
                            },
                        ))
                    }
                    RawUIMethodReturn::GetBufferContents(contents) => {
                        Some(PsValue::from_string_array(contents))
                    }
                    RawUIMethodReturn::SetForegroundColor
                    | RawUIMethodReturn::SetBackgroundColor
                    | RawUIMethodReturn::SetCursorPosition
                    | RawUIMethodReturn::SetWindowPosition
                    | RawUIMethodReturn::SetCursorSize
                    | RawUIMethodReturn::SetBufferSize
                    | RawUIMethodReturn::SetWindowSize
                    | RawUIMethodReturn::FlushInputBuffer
                    | RawUIMethodReturn::SetBufferContents
                    | RawUIMethodReturn::ScrollBufferContents => None, // void returns
                };
                (result, None)
            }
        };

        Ok((method_result, method_exception))
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
}

/// PowerShell Host UI method responses (return values)
#[derive(Debug, Clone, PartialEq)]
pub enum UIMethodReturn {
    ReadLine(String),
    ReadLineAsSecureString(Vec<u8>), // SecureString representation
    Write,                           // void return
    WriteLine,                       // void return
    WriteErrorLine,                  // void return
    WriteDebugLine,                  // void return
    WriteProgress,                   // void return
    WriteVerboseLine,                // void return
    WriteWarningLine,                // void return
    Prompt(Vec<String>),             // field values
    PromptForChoice(i32),            // selected choice index
    PromptForCredential(String, Vec<u8>), // username, password (SecureString)
}

/// PowerShell Host Raw UI method responses (return values)
#[derive(Debug, Clone, PartialEq)]
pub enum RawUIMethodReturn {
    GetForegroundColor(i32),
    SetForegroundColor, // void return
    GetBackgroundColor(i32),
    SetBackgroundColor,          // void return
    GetCursorPosition(i32, i32), // x, y
    SetCursorPosition,           // void return
    GetWindowPosition(i32, i32), // x, y
    SetWindowPosition,           // void return
    GetCursorSize(i32),
    SetCursorSize,                      // void return
    GetBufferSize(i32, i32),            // width, height
    SetBufferSize,                      // void return
    GetWindowSize(i32, i32),            // width, height
    SetWindowSize,                      // void return
    GetMaxWindowSize(i32, i32),         // width, height
    GetMaxPhysicalWindowSize(i32, i32), // width, height
    GetKeyAvailable(bool),
    ReadKey(i32, char, i32, i32), // virtual_key_code, character, control_key_state, key_down
    FlushInputBuffer,             // void return
    SetBufferContents,            // void return
    GetBufferContents(Vec<String>), // cell contents
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
            HostCallMethodWithParams::UIMethod(UIMethodParams::Write(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::Write),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteLine(_)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::WriteLine),
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
            HostCallMethodWithParams::UIMethod(UIMethodParams::WriteProgress(_, _)),
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
            HostCallMethodWithParams::UIMethod(UIMethodParams::Prompt(_, _)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::Prompt(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::PromptForChoice(_, _, _, _)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::PromptForChoice(_)),
        ) => true,
        (
            HostCallMethodWithParams::UIMethod(UIMethodParams::PromptForCredential(_, _, _, _)),
            HostCallMethodReturn::UIMethod(UIMethodReturn::PromptForCredential(_, _)),
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
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::SetBufferContents(_, _, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::SetBufferContents),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::GetBufferContents(_, _, _, _)),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferContents(_)),
        ) => true,
        (
            HostCallMethodWithParams::RawUIMethod(RawUIMethodParams::ScrollBufferContents(
                _,
                _,
                _,
                _,
                _,
                _,
                _,
            )),
            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::ScrollBufferContents),
        ) => true,

        // Error can match with any request
        (_, HostCallMethodReturn::Error(_)) => true,

        // No match
        _ => false,
    }
}
