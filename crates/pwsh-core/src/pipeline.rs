use crate::runspace_pool::PsInvocationState;

/// Represents a parameter value in business logic terms
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<ParameterValue>),
    Null,
}

/// Represents a single parameter for a command
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub value: Option<ParameterValue>,
}

/// Represents a single PowerShell command in business logic terms
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineCommand {
    pub command_text: String,
    pub is_script: bool,
    pub parameters: Vec<Parameter>,
}

impl PipelineCommand {
    pub fn new_script(script: String) -> Self {
        Self {
            command_text: script,
            is_script: true,
            parameters: Vec::new(),
        }
    }

    pub fn new_command(command: String) -> Self {
        Self {
            command_text: command,
            is_script: false,
            parameters: Vec::new(),
        }
    }

    pub fn add_parameter(&mut self, name: String, value: ParameterValue) {
        self.parameters.push(Parameter {
            name,
            value: Some(value),
        });
    }

    pub fn add_switch_parameter(&mut self, name: String) {
        self.parameters.push(Parameter { name, value: None });
    }

    pub(crate) fn new_output_stream() -> PipelineCommand {
        PipelineCommand::new_script("Out-String -Stream".to_string())
    }
}

/// Represents execution results in business terms
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub error_messages: Vec<String>,
    pub warning_messages: Vec<String>,
    pub debug_messages: Vec<String>,
    pub information_messages: Vec<String>,
    pub progress_records: Vec<protocol_powershell_remoting::ProgressRecord>,
    pub information_records: Vec<protocol_powershell_remoting::InformationRecord>,
}

/// Internal representation of a PowerShell pipeline's state and configuration.
/// This is owned and managed by the `RunspacePool`.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub(crate) state: PsInvocationState,
    pub(crate) commands: Vec<PipelineCommand>,
    pub(crate) results: ExecutionResult,
}

impl Pipeline {
    pub(crate) fn new() -> Self {
        Self {
            state: PsInvocationState::NotStarted,
            commands: Vec::new(),
            results: ExecutionResult::default(),
        }
    }

    pub(crate) fn add_information_record(
        &mut self,
        record: protocol_powershell_remoting::InformationRecord,
    ) {
        self.results.information_records.push(record);
    }

    pub(crate) fn add_progress_record(
        &mut self,
        record: protocol_powershell_remoting::ProgressRecord,
    ) {
        self.results.progress_records.push(record);
    }

    pub(crate) fn add_switch_parameter(&mut self, name: String) {
        if let Some(last_cmd) = self.commands.last_mut() {
            last_cmd.add_switch_parameter(name);
        } else {
            tracing::warn!("Attempted to add a switch parameter with no prior command.");
        }
    }

    pub(crate) fn add_command(&mut self, command: PipelineCommand) {
        self.commands.push(command);
    }
}

// Conversion methods to protocol types
impl From<ParameterValue> for protocol_powershell_remoting::PsValue {
    fn from(value: ParameterValue) -> Self {
        use protocol_powershell_remoting::{PsPrimitiveValue, PsValue};
        match value {
            ParameterValue::String(s) => PsValue::Primitive(PsPrimitiveValue::Str(s)),
            ParameterValue::Integer(i) => PsValue::Primitive(PsPrimitiveValue::I64(i)),
            ParameterValue::Boolean(b) => PsValue::Primitive(PsPrimitiveValue::Bool(b)),
            ParameterValue::Array(_arr) => {
                todo!("Convert array to PsValue")
            }
            ParameterValue::Null => PsValue::Primitive(PsPrimitiveValue::Nil),
        }
    }
}

impl From<&PipelineCommand> for protocol_powershell_remoting::Command {
    fn from(cmd: &PipelineCommand) -> Self {
        use protocol_powershell_remoting::{CommandParameter, PsPrimitiveValue, PsValue};

        // Convert parameters to CommandParameter
        let mut args = Vec::new();
        for param in &cmd.parameters {
            let param_value = match &param.value {
                Some(value) => value.clone().into(),
                None => {
                    // Switch parameter (no value) - use boolean true
                    PsValue::Primitive(PsPrimitiveValue::Bool(true))
                }
            };

            args.push(
                CommandParameter::builder()
                    .name(param.name.clone())
                    .value(param_value)
                    .build(),
            );
        }

        protocol_powershell_remoting::Command::builder()
            .cmd(&cmd.command_text)
            .is_script(cmd.is_script)
            .args(args)
            .build()
    }
}

impl Pipeline {
    /// Convert the business-level pipeline to a protocol-level PowerShellPipeline
    pub(crate) fn to_protocol_pipeline(
        &self,
    ) -> Result<
        protocol_powershell_remoting::messages::create_pipeline::PowerShellPipeline,
        crate::PwshCoreError,
    > {
        use protocol_powershell_remoting::{Command, Commands};

        // Convert all commands to protocol commands
        let protocol_commands: Vec<Command> = self.commands.iter().map(|cmd| cmd.into()).collect();

        // Use TryFrom to create Commands (handles empty check)
        let commands = Commands::try_from(protocol_commands)
            .map_err(crate::PwshCoreError::PowerShellRemotingError)?;

        Ok(
            protocol_powershell_remoting::messages::create_pipeline::PowerShellPipeline::builder()
                .is_nested(false)
                .redirect_shell_error_output_pipe(true)
                .cmds(commands)
                .build(),
        )
    }
}
