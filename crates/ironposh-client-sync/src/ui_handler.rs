use anyhow::Context;
use ironposh_client_core::connector::active_session::{self, PowershellOperations, UserEvent};
use ironposh_client_core::connector::UserOperation;
use ironposh_client_core::pipeline::PipelineCommand;
use ironposh_client_core::powershell::PipelineHandle;
use ironposh_terminal::{ReadOutcome, Terminal};
use std::io::Write;
use std::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use crate::types::{UIInputEvent, UiOp};

/// Handle user input for PowerShell commands (synchronous)
pub struct UIHanlder {
    user_request_tx: mpsc::Sender<UserOperation>,
    unified_rx: mpsc::Receiver<UIInputEvent>,
}

impl UIHanlder {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        unified_rx: mpsc::Receiver<UIInputEvent>,
    ) -> Self {
        Self {
            user_request_tx,
            unified_rx,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> anyhow::Result<()> {
        let mut io = terminal.stdio(); // stdio-like wrapper
        let mut pipeline: Option<PipelineHandle> = None;

        // boot pipeline as before
        self.user_request_tx.send(UserOperation::CreatePipeline {
            uuid: uuid::Uuid::new_v4(),
        })?;

        let UIInputEvent::UserEvent(UserEvent::PipelineCreated { powershell }) =
            self.unified_rx.recv()?
        else {
            anyhow::bail!("Expected initial PipelineCreated event");
        };

        info!(pipeline_id = %powershell.id(), "initial pipeline created");
        pipeline = Some(powershell);

        'ui: loop {
            match io.read_line("> ")? {
                ReadOutcome::Line(cmd) => {
                    let command = cmd.trim();
                    if command.eq_ignore_ascii_case("exit") {
                        break Ok(());
                    }
                    if command.is_empty() {
                        continue;
                    }

                    if let Some(pipeline_handle) = pipeline {
                        info!(pipeline_id = %pipeline_handle.id(), command = %command, "sending command to existing pipeline");
                        let pipeline_operations = [
                            UserOperation::OperatePipeline {
                                powershell: pipeline_handle,
                                operation: PowershellOperations::AddCommand {
                                    command: PipelineCommand::new_command(command.to_string()),
                                },
                            },
                            UserOperation::OperatePipeline {
                                powershell: pipeline_handle,
                                operation: PowershellOperations::AddCommand {
                                    command: PipelineCommand::new_output_stream(),
                                },
                            },
                            UserOperation::InvokePipeline {
                                powershell: pipeline_handle,
                            },
                        ];

                        for op in pipeline_operations {
                            self.user_request_tx
                                .send(op)
                                .context("Failed to send pipeline operation")?;
                        }
                    }
                }
                ReadOutcome::Interrupt => continue, // reprompt (like shells)
                ReadOutcome::Eof => break Ok(()),
            }

            'receive: while let Ok(event) = self.unified_rx.recv() {
                match event {
                    UIInputEvent::UiOp(op) => {
                        match op {
                            UiOp::Apply(ops) => {
                                for o in ops {
                                    io.apply_op(o);
                                }
                            }
                            UiOp::Print(s) => {
                                use std::io::Write;
                                writeln!(io, "{s}")?;
                            }
                        }
                        io.render()?;
                        continue 'receive;
                    }
                    UIInputEvent::UserEvent(user_event) => {
                        match user_event {
                            active_session::UserEvent::PipelineCreated { powershell } => {
                                info!(pipeline_id = %powershell.id(), "pipeline created");
                                pipeline = Some(powershell);
                                continue 'ui;
                            }
                            active_session::UserEvent::PipelineFinished { powershell } => {
                                info!(pipeline_id = %powershell.id(), "pipeline finished");
                                if let Some(current_pipeline) = &pipeline {
                                    if *current_pipeline == powershell {
                                        pipeline = None;
                                        self.user_request_tx
                                            .send(UserOperation::CreatePipeline {
                                                uuid: Uuid::new_v4(),
                                            })
                                            .expect("Failed to send create pipeline request");
                                    }
                                }
                                continue 'receive;
                            }
                            active_session::UserEvent::PipelineOutput { output, powershell } => {
                                info!(pipeline_id = %powershell.id(), ?output, "pipeline output");
                                if let Some(current_pipeline) = &pipeline {
                                    if *current_pipeline == powershell {
                                        match output.format_as_displyable_string() {
                                            Ok(o) => {
                                                let _ = writeln!(io, "{o}");
                                            }
                                            Err(e) => {
                                                let _ =
                                                    writeln!(io, "Error formatting output: {e}");
                                            }
                                        };
                                        let _ = io.render(); // best-effort
                                    }
                                }
                                continue 'receive;
                            }
                        }
                    }
                }
            }
        }
    }
}
