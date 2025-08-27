use uuid::Uuid;

use crate::{
    connector::UserOperation,
    pipeline::{Parameter, PipelineCommand},
};

/// A handle to a PowerShell pipeline managed by a `RunspacePool`.
///
/// This struct is a lightweight, copyable identifier for a specific pipeline.
/// All operations on the pipeline are performed via methods on the `RunspacePool`
/// that take this handle as an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle {
    pub(crate) id: uuid::Uuid,
}

impl PipelineHandle {
    /// Returns the unique identifier for this PowerShell handle.
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn invoke(&self) -> UserOperation {
        UserOperation::InvokePipeline { powershell: *self }
    }

    pub fn script(&self, script: String) -> UserOperation {
        UserOperation::OperatePipeline {
            powershell: *self,
            operation: crate::connector::active_session::PowershellOperations::AddCommand {
                command: PipelineCommand::new_script(script),
            },
        }
    }

    pub fn command(&self, command: String, params: Vec<Parameter>) -> UserOperation {
        let mut command = PipelineCommand::new_command(command);

        for params in params {
            command.add_parameter(params);
        }

        UserOperation::OperatePipeline {
            powershell: *self,
            operation: crate::connector::active_session::PowershellOperations::AddCommand {
                command,
            },
        }
    }

    pub fn command_builder(&self, command: String) -> SimpleCommandBuilder {
        SimpleCommandBuilder::new(*self, command)
    }
}

pub struct SimpleCommandBuilder {
    powershell: PipelineHandle,
    command: String,
    params: Vec<Parameter>,
}

impl SimpleCommandBuilder {
    pub fn new(powershell: PipelineHandle, command: String) -> Self {
        Self {
            powershell,
            command,
            params: Vec::new(),
        }
    }

    pub fn with_param(mut self, param: Parameter) -> Self {
        self.params.push(param);
        self
    }

    pub fn build(self) -> UserOperation {
        let mut command = PipelineCommand::new_command(self.command);

        for params in self.params {
            command.add_parameter(params);
        }

        UserOperation::OperatePipeline {
            powershell: self.powershell,
            operation: crate::connector::active_session::PowershellOperations::AddCommand {
                command,
            },
        }
    }
}
