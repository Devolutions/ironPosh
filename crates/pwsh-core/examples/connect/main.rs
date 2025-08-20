mod config;
mod connection;
mod http_client;
mod network;
mod types;
mod user_input;

use anyhow::Context;
use pwsh_core::connector::ActiveSessionOutput;
use pwsh_core::connector::active_session::UserEvent;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, instrument, warn};

use config::{create_connector_config, init_logging};
use connection::establish_connection;
use network::spawn_network_handler;
use tracing_subscriber::fmt::format;
use types::NextStep;
use user_input::spawn_user_input_handler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging()?;
    let _span = tracing::span!(tracing::Level::INFO, "main").entered();
    info!("Starting WinRM PowerShell client");

    // Create configuration and establish connection
    let config = create_connector_config();
    let (active_session, next_request) = establish_connection(config).await?;
    info!("Runspace pool is now open and ready for operations!");

    // Set up communication channels
    let (network_request_tx, network_request_rx) = mpsc::channel(2);
    let (network_response_tx, network_response_rx) = mpsc::channel(2);
    let (user_request_tx, user_request_rx) = mpsc::channel(2);

    // Spawn network handler
    let handle = spawn_network_handler(network_request_rx, network_response_tx);

    // Set up pipeline creation
    let (pipeline_tx, pipeline_rx) = oneshot::channel();
    let handle2 = spawn_user_input_handler(user_request_tx.clone(), pipeline_rx);

    // Send initial network request
    network_request_tx
        .send(next_request)
        .await
        .context("Failed to send initial request")?;

    // Run the main event loop
    run_event_loop(
        active_session,
        network_response_rx,
        user_request_rx,
        network_request_tx,
        Some(pipeline_tx),
    )
    .await
    .inspect_err(|e| error!("Error in main event loop: {}", e))?;

    info!("Exiting main function");
    handle.abort();
    handle2.abort();
    drop(_span);
    Ok(())
}

/// Main event loop that processes network responses and user requests
#[instrument(skip_all)]
async fn run_event_loop(
    mut active_session: pwsh_core::connector::active_session::ActiveSession,
    mut network_response_rx: mpsc::Receiver<pwsh_core::connector::http::HttpResponse<String>>,
    mut user_request_rx: mpsc::Receiver<pwsh_core::connector::UserOperation>,
    network_request_tx: mpsc::Sender<pwsh_core::connector::http::HttpRequest<String>>,
    mut pipeline_tx: Option<oneshot::Sender<pwsh_core::powershell::PowerShell>>,
) -> anyhow::Result<()> {
    loop {
        let next_step = tokio::select! {
            network_response = network_response_rx.recv() => {
                if let Some(response) = network_response {
                    NextStep::NetworkResponse(response)
                } else {
                    error!("No response received from server");
                    return Err(anyhow::anyhow!("No response received from server"));
                }
            },
            user_request = user_request_rx.recv() => {
                if let Some(user_request) = user_request {
                    NextStep::UserRequest(user_request)
                } else {
                    error!("No user request received");
                    return Err(anyhow::anyhow!("No user request received"));
                }
            },
        };

        info!("Processing next step: {next_step}");

        let step_results = match next_step {
            NextStep::NetworkResponse(http_response) => {
                info!(
                    "Processing network response with body length: {}",
                    http_response.body.as_ref().map(|b| b.len()).unwrap_or(0)
                );

                active_session
                    .accept_server_response(http_response)
                    .map_err(|e| {
                        error!("Failed to accept server response: {:#}", e);
                        e
                    })
                    .context("Failed to accept server response")?
            }
            NextStep::UserRequest(user_operation) => {
                info!("Processing user operation: {:?}", user_operation);

                vec![
                    active_session
                        .accept_client_operation(user_operation)
                        .map_err(|e| {
                            error!("Failed to accept user operation: {:#}", e);
                            e
                        })
                        .context("Failed to accept user operation")?,
                ]
            }
        };

        info!(?step_results, "Received server response, processing...");

        for step_result in step_results {
            info!(?step_result, "Processing step result");
            match step_result {
                ActiveSessionOutput::SendBack(http_requests) => {
                    for http_request in http_requests {
                        network_request_tx
                            .send(http_request)
                            .await
                            .context("Failed to send HTTP request")?;
                    }
                }
                ActiveSessionOutput::SendBackError(e) => {
                    error!("Error in session step: {}", e);
                    return Err(anyhow::anyhow!("Session step failed: {}", e));
                }
                ActiveSessionOutput::UserEvent(event) => match event {
                    UserEvent::PipelineCreated { powershell } => {
                        info!("Pipeline created: {:?}", powershell);
                        let sent = pipeline_tx.take().map(|tx| tx.send(powershell));
                        if let Some(Err(_)) = sent {
                            error!("Failed to send pipeline through channel");
                            return Err(anyhow::anyhow!("Failed to send pipeline through channel"));
                        }
                    }
                },
                ActiveSessionOutput::HostCall(host_call) => {
                    info!("Received host call: {:?}", host_call);
                    let method = host_call.get_param().map_err(|e| {
                        error!("Failed to get host call parameters: {:#}", e);
                        e
                    })?;
                    info!("Handling host call method: {:?}", method);

                    // For now, we'll just log the host call but not implement full handling
                    // This prevents the todo!() panic from occurring
                    warn!("Host call received but not fully implemented: {:?}", method);

                    // We should implement proper host call handling here
                    // For WriteProgress calls, we typically don't need to send a response
                    // since they are void methods
                }
                ActiveSessionOutput::OperationSuccess => {
                    info!("Operation completed successfully");
                }
            }
        }
    }
}
