use super::{HostError, HostResult, PSHostRawUserInterface};
use protocol_powershell_remoting::{ComplexObject, PsValue};
use std::collections::HashMap;

/// Defines the properties and facilities provided by a hosting application
/// deriving from PSHost that offers dialog-oriented and line-oriented
/// interactive features.
/// Corresponds to System.Management.Automation.Host.PSHostUserInterface
pub trait PSHostUserInterface {
    /// Reads a line of characters from a user
    /// MI: 11
    fn read_line(&mut self) -> HostResult<String>;

    /// Reads a line of characters from a user, with the user input not echoed
    /// MI: 12
    fn read_line_as_secure_string(&mut self) -> HostResult<String>;

    /// Writes specified characters on the hosting application
    /// MI: 13
    fn write(&mut self, value: &str) -> HostResult<()>;

    /// Writes the specified characters with the specified foreground and background color
    /// MI: 14
    fn write_with_color(
        &mut self,
        value: &str,
        foreground_color: i32,
        background_color: i32,
    ) -> HostResult<()>;

    /// Writes a carriage return on the hosting application
    /// MI: 15
    fn write_line(&mut self) -> HostResult<()>;

    /// Writes the specified line on the hosting application
    /// MI: 16
    fn write_line_str(&mut self, value: &str) -> HostResult<()>;

    /// Writes the specified line with the specified foreground and background color
    /// MI: 17
    fn write_line_with_color(
        &mut self,
        value: &str,
        foreground_color: i32,
        background_color: i32,
    ) -> HostResult<()>;

    /// Writes a line to the error display of the hosting application
    /// MI: 18
    fn write_error_line(&mut self, message: &str) -> HostResult<()>;

    /// Writes a line to the debug display of the hosting application
    /// MI: 19
    fn write_debug_line(&mut self, message: &str) -> HostResult<()>;

    /// Displays a progress record on the hosting application
    /// MI: 20
    fn write_progress(&mut self, source_id: i32, record: &str) -> HostResult<()>;

    /// Writes a line on the verbose display of the hosting application
    /// MI: 21
    fn write_verbose_line(&mut self, message: &str) -> HostResult<()>;

    /// Writes a line on the warning display of the hosting application
    /// MI: 22
    fn write_warning_line(&mut self, message: &str) -> HostResult<()>;

    /// Prompts the user with a set of choices
    /// MI: 23
    fn prompt(
        &mut self,
        caption: &str,
        message: &str,
        descriptions: &[ComplexObject],
    ) -> HostResult<HashMap<String, PsValue>>;

    /// Prompts the user for entering credentials with the specified parameters
    /// MI: 24
    fn prompt_for_credential(
        &mut self,
        caption: &str,
        message: &str,
        user_name: &str,
        target_name: &str,
    ) -> HostResult<ComplexObject>; // PSCredential

    /// Prompts the user for entering credentials with extended options
    /// MI: 25
    fn prompt_for_credential_with_options(
        &mut self,
        caption: &str,
        message: &str,
        user_name: &str,
        target_name: &str,
        allowed_credential_types: i32,
        options: i32,
    ) -> HostResult<ComplexObject>; // PSCredential

    /// Displays a list of choices to the user and returns the index of the selected option
    /// MI: 26
    fn prompt_for_choice(
        &mut self,
        caption: &str,
        message: &str,
        choices: &[ComplexObject],
        default_choice: i32,
    ) -> HostResult<i32>;

    /// Gets the raw UI interface implementation (can be None for hosts without raw UI support)
    fn get_raw_ui(&self) -> Option<&dyn PSHostRawUserInterface>;

    /// Gets the mutable raw UI interface implementation (can be None for hosts without raw UI support)
    fn get_raw_ui_mut(&mut self) -> Option<&mut dyn PSHostRawUserInterface>;

    /// Runs a UI method call based on the method identifier and parameters
    fn run_method(
        &mut self,
        method_id: i32,
        _method_name: &str,
        parameters: &[PsValue],
    ) -> HostResult<Option<PsValue>> {
        match method_id {
            11 => {
                let result = self.read_line()?;
                Ok(Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(result),
                )))
            }
            12 => {
                let result = self.read_line_as_secure_string()?;
                Ok(Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(result),
                )))
            }
            13 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(value),
                )) = parameters.first()
                {
                    self.write(value)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            14 => {
                if let (
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::I32(
                        fg,
                    ))),
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::I32(
                        bg,
                    ))),
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(
                        value,
                    ))),
                ) = (parameters.first(), parameters.get(1), parameters.get(2))
                {
                    self.write_with_color(value, *fg, *bg)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            15 => {
                self.write_line()?;
                Ok(None)
            }
            16 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(value),
                )) = parameters.first()
                {
                    self.write_line_str(value)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            17 => {
                if let (
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::I32(
                        fg,
                    ))),
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::I32(
                        bg,
                    ))),
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(
                        value,
                    ))),
                ) = (parameters.first(), parameters.get(1), parameters.get(2))
                {
                    self.write_line_with_color(value, *fg, *bg)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            18 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(message),
                )) = parameters.first()
                {
                    self.write_error_line(message)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            19 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(message),
                )) = parameters.first()
                {
                    self.write_debug_line(message)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            20 => {
                if let (
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::I32(
                        source_id,
                    ))),
                    Some(PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(
                        record,
                    ))),
                ) = (parameters.first(), parameters.get(1))
                {
                    self.write_progress(*source_id, record)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            21 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(message),
                )) = parameters.first()
                {
                    self.write_verbose_line(message)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            22 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(message),
                )) = parameters.first()
                {
                    self.write_warning_line(message)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            23 => {
                // Prompt - complex implementation needed
                Err(HostError::NotImplemented)
            }
            24 => {
                // PromptForCredential - complex implementation needed
                Err(HostError::NotImplemented)
            }
            25 => {
                // PromptForCredentialWithOptions - complex implementation needed
                Err(HostError::NotImplemented)
            }
            26 => {
                // PromptForChoice - complex implementation needed
                Err(HostError::NotImplemented)
            }
            27..=51 => {
                // Raw UI methods
                if let Some(raw_ui) = self.get_raw_ui_mut() {
                    raw_ui.run_method(method_id, _method_name, parameters)
                } else {
                    Err(HostError::NotImplemented)
                }
            }
            _ => Err(HostError::NotImplemented),
        }
    }
}
