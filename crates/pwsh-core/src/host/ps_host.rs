use super::{HostError, HostResult, PSHostUserInterface};
use protocol_powershell_remoting::PsValue;
use uuid::Uuid;

/// Represents the main PowerShell host interface
/// This trait defines the properties and facilities provided by an application
/// hosting a RunspacePool, corresponding to the System.Management.Automation.Host.PSHost class
pub trait PSHost {
    /// Gets the hosting application identification in a user-friendly fashion
    /// MI: 1
    fn get_name(&self) -> HostResult<Option<String>>;

    /// Gets the version number of the hosting application
    /// MI: 2
    fn get_version(&self) -> HostResult<Option<String>>;

    /// Gets a GUID that uniquely identifies the hosting application
    /// MI: 3
    fn get_instance_id(&self) -> HostResult<Uuid>;

    /// Gets the host's culture
    /// MI: 4
    fn get_current_culture(&self) -> HostResult<Option<String>>;

    /// Gets the host's UI culture
    /// MI: 5
    fn get_current_ui_culture(&self) -> HostResult<Option<String>>;

    /// Shuts down the hosting application and closes the current runspace
    /// MI: 6
    fn set_should_exit(&mut self, exit_code: i32) -> HostResult<()>;

    /// Interrupts the current pipeline and starts a nested pipeline
    /// MI: 7
    fn enter_nested_prompt(&mut self) -> HostResult<()>;

    /// Stops the nested pipeline and resumes the current pipeline
    /// MI: 8
    fn exit_nested_prompt(&mut self) -> HostResult<()>;

    /// Called by an application to indicate that it is executing a command line application
    /// MI: 9
    fn notify_begin_application(&mut self) -> HostResult<()>;

    /// Called by an application to indicate that it has finished executing a command line application
    /// MI: 10
    fn notify_end_application(&mut self) -> HostResult<()>;

    /// Gets the UI interface implementation (can be None for headless hosts)
    fn get_ui(&self) -> Option<&dyn PSHostUserInterface>;

    /// Gets the mutable UI interface implementation (can be None for headless hosts)
    fn get_ui_mut(&mut self) -> Option<&mut dyn PSHostUserInterface>;

    /// Runs a host method call based on the method identifier and parameters
    fn run_method(
        &mut self,
        method_id: i32,
        _method_name: &str,
        parameters: &[PsValue],
    ) -> HostResult<Option<PsValue>> {
        match method_id {
            1 => Ok(self.get_name()?.map(|s| {
                PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(s))
            })),
            2 => Ok(self.get_version()?.map(|s| {
                PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(s))
            })),
            3 => {
                let id = self.get_instance_id()?;
                Ok(Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(id.to_string()),
                )))
            }
            4 => Ok(self.get_current_culture()?.map(|s| {
                PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(s))
            })),
            5 => Ok(self.get_current_ui_culture()?.map(|s| {
                PsValue::Primitive(protocol_powershell_remoting::PsPrimitiveValue::Str(s))
            })),
            6 => {
                if let Some(exit_code) = parameters.first() {
                    if let PsValue::Primitive(
                        protocol_powershell_remoting::PsPrimitiveValue::I32(code),
                    ) = exit_code
                    {
                        self.set_should_exit(*code)?;
                        Ok(None)
                    } else {
                        Err(HostError::InvalidParameters)
                    }
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            7 => {
                self.enter_nested_prompt()?;
                Ok(None)
            }
            8 => {
                self.exit_nested_prompt()?;
                Ok(None)
            }
            9 => {
                self.notify_begin_application()?;
                Ok(None)
            }
            10 => {
                self.notify_end_application()?;
                Ok(None)
            }
            11..=26 => {
                // UI methods (11-26)
                if let Some(ui) = self.get_ui_mut() {
                    ui.run_method(method_id, _method_name, parameters)
                } else {
                    Err(HostError::NotImplemented)
                }
            }
            27..=51 => {
                // Raw UI methods (27-51)
                if let Some(ui) = self.get_ui_mut() {
                    if let Some(raw_ui) = ui.get_raw_ui_mut() {
                        raw_ui.run_method(method_id, _method_name, parameters)
                    } else {
                        Err(HostError::NotImplemented)
                    }
                } else {
                    Err(HostError::NotImplemented)
                }
            }
            _ => Err(HostError::NotImplemented),
        }
    }
}
