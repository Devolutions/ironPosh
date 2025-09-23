use anyhow::Context;
use ironposh_client_core::connector::active_session::{self, PowershellOperations};
use ironposh_client_core::connector::UserOperation;
use ironposh_client_core::pipeline::PipelineCommand;
use ironposh_client_core::powershell::PipelineHandle;
use ironposh_terminal::{ReadOutcome, StdTerm, Terminal};
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::types::UiOp;

/// Handle user input for PowerShell commands (synchronous)
pub struct UIHanlder {
    user_request_tx: mpsc::Sender<UserOperation>,
    user_event_rx: mpsc::Receiver<active_session::UserEvent>,
    ui_rx: mpsc::Receiver<UiOp>,
}

impl UIHanlder {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        user_event_rx: mpsc::Receiver<active_session::UserEvent>,
        ui_rx: mpsc::Receiver<UiOp>,
    ) -> Self {
        Self {
            user_request_tx,
            user_event_rx,
            ui_rx,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> anyhow::Result<()> {
        let mut io = terminal.stdio(); // stdio-like wrapper
        let mut pipeline: Option<PipelineHandle> = None;

        // boot pipeline as before
        self.user_request_tx.send(UserOperation::CreatePipeline {
            uuid: uuid::Uuid::new_v4(),
        })?;

        loop {
            // 4a) drain UI ops quickly (paint from main loop HostCalls)
            while let Ok(op) = self.ui_rx.try_recv() {
                match op {
                    UiOp::Apply(ops) => {
                        for o in ops {
                            io.apply_op(o);
                        }
                        io.render()?; // throttled internally
                    }
                    UiOp::Print(s) => {
                        use std::io::Write;
                        writeln!(io, "{s}")?;
                    }
                }
            }

            // 4b) process PowerShell events (pipeline output etc.)
            match self.process_user_events(&mut pipeline, &mut io) {
                PipelineOperated::Continue => {}
                PipelineOperated::KeepGoing => { /* pipeline (re)created, just proceed */ }
            }

            // 4c) prompt + read a line (Ctrl+C / Ctrl+D handled)
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
        }
    }

    #[instrument(skip_all)]
    fn process_user_events(
        &mut self,
        pipeline: &mut Option<PipelineHandle>,
        io: &mut StdTerm<'_>,
    ) -> PipelineOperated {
        while let Ok(event) = self.user_event_rx.recv_timeout(Duration::from_millis(100)) {
            match event {
                ironposh_client_core::connector::active_session::UserEvent::PipelineCreated {
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(), "pipeline created");
                    *pipeline = Some(powershell);
                    return PipelineOperated::KeepGoing;
                }
                ironposh_client_core::connector::active_session::UserEvent::PipelineFinished {
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(), "pipeline finished");
                    if let Some(current_pipeline) = pipeline {
                        if *current_pipeline == powershell {
                            *pipeline = None;
                            self.user_request_tx
                                .send(UserOperation::CreatePipeline {
                                    uuid: Uuid::new_v4(),
                                })
                                .expect("Failed to send create pipeline request");
                        }
                    }
                }
                ironposh_client_core::connector::active_session::UserEvent::PipelineOutput {
                    output,
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(),?output, "pipeline output");
                    if let Some(current_pipeline) = pipeline {
                        if *current_pipeline == powershell {
                            match output.format_as_displyable_string() {
                                Ok(o) => {
                                    let _ = writeln!(io, "{o}");
                                }
                                Err(e) => {
                                    let _ = writeln!(io, "Error formatting output: {e}");
                                }
                            };

                            let _ = io.render(); // best-effort
                        }
                    }
                }
            }
        }
        PipelineOperated::Continue
    }
}

pub enum PipelineOperated {
    Continue,
    KeepGoing,
}
