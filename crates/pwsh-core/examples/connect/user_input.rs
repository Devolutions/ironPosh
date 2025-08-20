use pwsh_core::connector::active_session::PowershellOperations;
use pwsh_core::pipeline::PipelineCommand;
use pwsh_core::powershell::PipelineHandle;
use pwsh_core::{connector::UserOperation, powershell::PipelineOutputType};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tracing::{Instrument, error, info, info_span};

/// Handle user input for PowerShell commands
pub async fn handle_user_input(
    user_request_tx: mpsc::Sender<UserOperation>,
    pipeline: PipelineHandle,
) {
    info!("Pipeline ready! Enter PowerShell commands (type 'exit' to quit):");

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut line = String::new();

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let command = line.trim().to_string();
                if command.to_lowercase() == "exit" {
                    info!("Exiting...");
                    break;
                }
                if !command.is_empty() {
                    // Add the script to the pipeline
                    if let Err(e) = user_request_tx
                        .send(UserOperation::OperatePipeline {
                            powershell: pipeline,
                            operation: PowershellOperations::AddCommand {
                                command: PipelineCommand::new_script(command),
                            },
                        })
                        .await
                    {
                        error!("Failed to send operation: {}", e);
                        break;
                    }

                    // Invoke the pipeline
                    if let Err(e) = user_request_tx
                        .send(UserOperation::InvokePipeline {
                            powershell: pipeline,
                            output_type: PipelineOutputType::Streamed,
                        })
                        .await
                    {
                        error!("Failed to send invoke: {}", e);
                        break;
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

/// Spawn user input handler task
pub fn spawn_user_input_handler(
    user_request_tx: mpsc::Sender<UserOperation>,
    pipeline_rx: tokio::sync::oneshot::Receiver<PipelineHandle>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        async move {
            info!("Creating initial pipeline...");
            if let Err(e) = user_request_tx.send(UserOperation::CreatePipeline).await {
                error!("Failed to send create pipeline request: {}", e);
                return;
            }

            // Wait for pipeline to be created
            match pipeline_rx.await {
                Ok(pipeline) => {
                    handle_user_input(user_request_tx, pipeline).await;
                }
                Err(e) => {
                    error!("Failed to receive pipeline: {}", e);
                }
            }
        }
        .instrument(info_span!("UserInputHandler")),
    )
}
