use super::{
    error::HostError,
    methods::{HostCallMethodWithParams, HostMethodParams, RawUIMethodParams, UIMethodParams},
};

/// Convert HostCallRequest to HostCallMethodWithParams based on method_id and parameters
impl TryFrom<&super::HostCallRequest> for HostCallMethodWithParams {
    type Error = HostError;

    fn try_from(call: &super::HostCallRequest) -> Result<Self, Self::Error> {
        // PowerShell method IDs based on the protocol specification
        // Host methods: 1-10, UI methods: 11-30, RawUI methods: 31-60
        match call.method_id {
            // Host methods (1-10)
            1 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::GetName,
            )),
            2 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::GetVersion,
            )),
            3 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::GetInstanceId,
            )),
            4 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::GetCurrentCulture,
            )),
            5 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::GetCurrentUICulture,
            )),
            6 => {
                let exit_code = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::HostMethod(
                    HostMethodParams::SetShouldExit(exit_code),
                ))
            }
            7 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::EnterNestedPrompt,
            )),
            8 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::ExitNestedPrompt,
            )),
            9 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::NotifyBeginApplication,
            )),
            10 => Ok(HostCallMethodWithParams::HostMethod(
                HostMethodParams::NotifyEndApplication,
            )),

            // UI methods (11-30)
            11 => Ok(HostCallMethodWithParams::UIMethod(UIMethodParams::ReadLine)),
            12 => Ok(HostCallMethodWithParams::UIMethod(
                UIMethodParams::ReadLineAsSecureString,
            )),
            13 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(UIMethodParams::Write(
                    value,
                )))
            }
            14 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteLine(value),
                ))
            }
            15 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteErrorLine(value),
                ))
            }
            16 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteDebugLine(value),
                ))
            }
            17 => {
                let source_id = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i64())
                    .ok_or(HostError::InvalidParameters)?;
                let record = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteProgress(source_id, record),
                ))
            }
            18 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteVerboseLine(value),
                ))
            }
            19 => {
                let value = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::WriteWarningLine(value),
                ))
            }
            20 => {
                let caption = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let descriptions = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_string_array())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(UIMethodParams::Prompt(
                    caption,
                    descriptions,
                )))
            }
            21 => {
                let caption = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let message = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let choices = call
                    .parameters
                    .get(2)
                    .and_then(|p| p.as_string_array())
                    .ok_or(HostError::InvalidParameters)?;
                let default_choice = call
                    .parameters
                    .get(3)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::PromptForChoice(caption, message, choices, default_choice),
                ))
            }
            22 => {
                let caption = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let message = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let user_name = call
                    .parameters
                    .get(2)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                let target_name = call
                    .parameters
                    .get(3)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::UIMethod(
                    UIMethodParams::PromptForCredential(caption, message, user_name, target_name),
                ))
            }

            // RawUI methods (31-60)
            31 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetForegroundColor,
            )),
            32 => {
                let color = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetForegroundColor(color),
                ))
            }
            33 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetBackgroundColor,
            )),
            34 => {
                let color = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetBackgroundColor(color),
                ))
            }
            35 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetCursorPosition,
            )),
            36 => {
                let x = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let y = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetCursorPosition(x, y),
                ))
            }
            37 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetWindowPosition,
            )),
            38 => {
                let x = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let y = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetWindowPosition(x, y),
                ))
            }
            39 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetCursorSize,
            )),
            40 => {
                let percentage = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetCursorSize(percentage),
                ))
            }
            41 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetBufferSize,
            )),
            42 => {
                let width = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let height = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetBufferSize(width, height),
                ))
            }
            43 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetWindowSize,
            )),
            44 => {
                let width = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let height = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetWindowSize(width, height),
                ))
            }
            45 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetMaxWindowSize,
            )),
            46 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetMaxPhysicalWindowSize,
            )),
            47 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::GetKeyAvailable,
            )),
            48 => {
                let options = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::ReadKey(options),
                ))
            }
            49 => Ok(HostCallMethodWithParams::RawUIMethod(
                RawUIMethodParams::FlushInputBuffer,
            )),
            50 => {
                let x = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let y = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let contents = call
                    .parameters
                    .get(2)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::SetBufferContents(x, y, contents),
                ))
            }
            51 => {
                let x = call
                    .parameters
                    .first()
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let y = call
                    .parameters
                    .get(1)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let width = call
                    .parameters
                    .get(2)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                let height = call
                    .parameters
                    .get(3)
                    .and_then(|p| p.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                Ok(HostCallMethodWithParams::RawUIMethod(
                    RawUIMethodParams::GetBufferContents(x, y, width, height),
                ))
            }
            52 => {
                // ScrollBufferContents has complex parameters - simplified for now
                let params: Vec<i32> = call
                    .parameters
                    .iter()
                    .take(6)
                    .filter_map(|p| p.as_i32())
                    .collect();
                let fill = call
                    .parameters
                    .get(6)
                    .and_then(|p| p.as_string())
                    .ok_or(HostError::InvalidParameters)?;

                if params.len() == 6 {
                    Ok(HostCallMethodWithParams::RawUIMethod(
                        RawUIMethodParams::ScrollBufferContents(
                            params[0], params[1], params[2], params[3], params[4], params[5], fill,
                        ),
                    ))
                } else {
                    Err(HostError::InvalidParameters)
                }
            }

            _ => Err(HostError::NotImplemented),
        }
    }
}
