use anyhow::Context;
use ironposh_client_core::connector::active_session::PowershellOperations;
use ironposh_client_core::connector::UserOperation;
use ironposh_client_core::pipeline::PipelineCommand;
use ironposh_client_core::powershell::PipelineHandle;
use ironposh_psrp::PipelineOutput;
use regex::Regex;
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Handle user input for PowerShell commands (synchronous)
pub struct UserInputHandler {
    user_request_tx: mpsc::Sender<UserOperation>,
    user_event_rx: mpsc::Receiver<ironposh_client_core::connector::active_session::UserEvent>,
}

impl UserInputHandler {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        user_event_rx: mpsc::Receiver<ironposh_client_core::connector::active_session::UserEvent>,
    ) -> Self {
        Self {
            user_request_tx,
            user_event_rx,
        }
    }

    #[instrument(skip_all, name = "user_input_handler")]
    pub fn run(&mut self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut pipeline: Option<PipelineHandle> = None;

        info!("starting user input handler");
        self.user_request_tx
            .send(UserOperation::CreatePipeline {
                uuid: uuid::Uuid::new_v4(),
            })
            .expect("Failed to send create pipeline request");

        loop {
            // Check for user events
            match self.process_user_events(&mut pipeline) {
                PipelineOperated::Continue => continue,
                PipelineOperated::KeepGoing => {}
            }

            println!("> ");
            stdout.flush().unwrap();
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) => break Ok(()),
                Ok(_) => {
                    let command = line.trim().to_string();
                    if command.to_lowercase() == "exit" {
                        info!("user requested exit");
                        break Ok(());
                    }

                    if command.is_empty() {
                        continue;
                    }
                    // Ensure we have a pipeline before executing the command

                    if let Some(pipeline_handle) = pipeline {
                        // Add the script to the pipeline
                        let _ = self
                            .user_request_tx
                            .send(UserOperation::OperatePipeline {
                                powershell: pipeline_handle,
                                operation: PowershellOperations::AddCommand {
                                    command: PipelineCommand::new_command(command),
                                },
                            })
                            .context("Failed to send add command operation to pipeline")?;

                        let _ = self
                            .user_request_tx
                            .send(UserOperation::OperatePipeline {
                                powershell: pipeline_handle,
                                operation: PowershellOperations::AddCommand {
                                    command: PipelineCommand::new_output_stream(),
                                },
                            })
                            .context("Failed to send add output stream operation to pipeline")?;

                        // Invoke the pipeline
                        let _ = self
                            .user_request_tx
                            .send(UserOperation::InvokePipeline {
                                powershell: pipeline_handle,
                            })
                            .context("Failed to send invoke pipeline operation")?;
                    }
                }
                Err(e) => {
                    error!(error = %e, "failed to read input");
                    break Err(e.into());
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn process_user_events(&mut self, pipeline: &mut Option<PipelineHandle>) -> PipelineOperated {
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
                                Ok(o) => print!("{}", o),
                                Err(e) => eprintln!("Error formatting output: {}", e),
                            };
                            // Flush stdout to ensure output is displayed immediately
                            std::io::stdout().flush().unwrap();
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
