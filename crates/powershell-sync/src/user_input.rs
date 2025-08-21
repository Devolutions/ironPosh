use pwsh_core::connector::active_session::PowershellOperations;
use pwsh_core::pipeline::PipelineCommand;
use pwsh_core::powershell::PipelineHandle;
use pwsh_core::{connector::UserOperation, powershell::PipelineOutputType};
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;
use tracing::{error, info, info_span};

/// Handle user input for PowerShell commands (synchronous)
pub struct UserInputHandler {
    user_request_tx: mpsc::Sender<UserOperation>,
    user_event_rx: mpsc::Receiver<pwsh_core::connector::active_session::UserEvent>,
}

impl UserInputHandler {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        user_event_rx: mpsc::Receiver<pwsh_core::connector::active_session::UserEvent>,
    ) -> Self {
        Self {
            user_request_tx,
            user_event_rx,
        }
    }

    pub fn run(&mut self) {
        let _span = info_span!("UserInputHandler").entered();
        
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut pipeline: Option<PipelineHandle> = None;

        loop {
            // Check for user events
            self.process_user_events(&mut pipeline);

            print!("> ");
            stdout.flush().unwrap();
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let command = line.trim().to_string();
                    if command.to_lowercase() == "exit" {
                        info!("Exiting...");
                        break;
                    }
                    if !command.is_empty() {
                        // Ensure we have a pipeline before executing the command
                        if pipeline.is_none() {
                            println!("Creating pipeline...");
                            if let Err(e) = self.user_request_tx.send(UserOperation::CreatePipeline) {
                                error!("Failed to send create pipeline request: {}", e);
                                break;
                            }
                            
                            // Wait for pipeline creation
                            pipeline = self.wait_for_pipeline_creation();
                            if pipeline.is_none() {
                                println!("Failed to create pipeline. Skipping command.");
                                continue;
                            }
                        }

                        if let Some(pipeline_handle) = pipeline {
                            // Add the script to the pipeline
                            if let Err(e) =
                                self.user_request_tx.send(UserOperation::OperatePipeline {
                                    powershell: pipeline_handle,
                                    operation: PowershellOperations::AddCommand {
                                        command: PipelineCommand::new_script(command),
                                    },
                                })
                            {
                                error!("Failed to send operation: {}", e);
                                break;
                            }

                            // Invoke the pipeline
                            if let Err(e) =
                                self.user_request_tx.send(UserOperation::InvokePipeline {
                                    powershell: pipeline_handle,
                                    output_type: PipelineOutputType::Streamed,
                                })
                            {
                                error!("Failed to send invoke: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read input: {}", e);
                    break;
                }
            }
        }
    }

    fn process_user_events(&mut self, pipeline: &mut Option<PipelineHandle>) {
        while let Ok(event) = self.user_event_rx.try_recv() {
            match event {
                pwsh_core::connector::active_session::UserEvent::PipelineCreated {
                    powershell,
                } => {
                    info!("Pipeline created: {:?}", powershell);
                    *pipeline = Some(powershell);
                    println!("Pipeline created and ready!");
                }
                pwsh_core::connector::active_session::UserEvent::PipelineFinished {
                    powershell,
                } => {
                    info!("Pipeline finished: {:?}", powershell);
                    if let Some(current_pipeline) = pipeline {
                        if *current_pipeline == powershell {
                            println!("Current pipeline has finished execution.");
                            *pipeline = None;
                        }
                    }
                    println!("Pipeline execution finished.");
                }
            }
        }
    }

    fn wait_for_pipeline_creation(&mut self) -> Option<PipelineHandle> {
        use std::time::{Duration, Instant};
        
        let timeout = Duration::from_secs(10); // 10 second timeout
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            match self.user_event_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => match event {
                    pwsh_core::connector::active_session::UserEvent::PipelineCreated {
                        powershell,
                    } => {
                        info!("Pipeline created: {:?}", powershell);
                        println!("Pipeline created and ready!");
                        return Some(powershell);
                    }
                    pwsh_core::connector::active_session::UserEvent::PipelineFinished {
                        powershell: _,
                    } => {
                        // Ignore pipeline finished events during creation wait
                        continue;
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Continue waiting
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("User event channel disconnected while waiting for pipeline");
                    return None;
                }
            }
        }
        
        error!("Timeout waiting for pipeline creation");
        None
    }
}
