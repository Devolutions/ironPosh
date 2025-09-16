use ironposh_psrp::{CommandParameter, PsValue};

use crate::runspace_pool::PsInvocationState;

/// Represents a single parameter for a command
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Named { name: String, value: PsValue },
    Positional { value: PsValue },
    Switch { name: String, value: bool },
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

    pub fn add_parameter(&mut self, params: Parameter) {
        self.parameters.push(params);
    }

    pub fn with_parameter(mut self, params: Parameter) -> Self {
        self.parameters.push(params);
        self
    }

    pub fn new_output_stream() -> PipelineCommand {
        let mut command = PipelineCommand::new_command("Out-String".to_string());
        command.add_parameter(Parameter::Switch {
            name: "Stream".to_string(),
            value: true,
        });
        command
    }
}

/// Represents execution results in business terms
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub error_messages: Vec<String>,
    pub warning_messages: Vec<String>,
    pub debug_messages: Vec<String>,
    pub information_messages: Vec<String>,
    pub progress_records: Vec<ironposh_psrp::ProgressRecord>,
    pub information_records: Vec<ironposh_psrp::InformationRecord>,
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

    pub(crate) fn add_information_record(&mut self, record: ironposh_psrp::InformationRecord) {
        self.results.information_records.push(record);
    }

    pub(crate) fn add_progress_record(&mut self, record: ironposh_psrp::ProgressRecord) {
        self.results.progress_records.push(record);
    }

    pub(crate) fn add_command(&mut self, command: PipelineCommand) {
        self.commands.push(command);
    }
}

impl Pipeline {
    /// Convert the business-level pipeline to a protocol-level PowerShellPipeline
    pub(crate) fn to_protocol_pipeline(
        &self,
    ) -> Result<ironposh_psrp::messages::create_pipeline::PowerShellPipeline, crate::PwshCoreError>
    {
        use ironposh_psrp::Command;

        // Convert all commands to protocol commands
        let protocol_commands: Vec<Command> = self
            .commands
            .iter()
            .map(|cmd| {
                ironposh_psrp::Command::builder()
                    .cmd(cmd.command_text.clone())
                    .is_script(cmd.is_script)
                    .args(
                        cmd.parameters
                            .iter()
                            .map(|param| match param {
                                Parameter::Named { name, value } => {
                                    CommandParameter::named(name.to_string(), value.clone())
                                }
                                Parameter::Positional { value } => {
                                    CommandParameter::positional(value.clone())
                                }
                                Parameter::Switch { name, value } => {
                                    CommandParameter::named(name.to_string(), *value)
                                }
                            })
                            .collect(),
                    )
                    .build()
            })
            .collect();

        Ok(
            ironposh_psrp::messages::create_pipeline::PowerShellPipeline::builder()
                .is_nested(false)
                .redirect_shell_error_output_pipe(true)
                .cmds(protocol_commands)
                .build(),
        )
    }
}
