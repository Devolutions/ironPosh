mod config;
mod connection;
mod http_client;
mod network;
mod types;
mod user_input;

use anyhow::Context;
use protocol_powershell_remoting::{PipelineOutput, PsPrimitiveValue, PsValue};
use pwsh_core::connector::ActiveSessionOutput;
use pwsh_core::connector::active_session::UserEvent;
use regex::Regex;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, instrument, warn};

use config::{create_connector_config, init_logging};
use connection::establish_connection;
use network::spawn_network_handler;
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
    mut pipeline_tx: Option<oneshot::Sender<pwsh_core::powershell::PipelineHandle>>,
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
                    NextStep::UserRequest(Box::new(user_request))
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
                        .accept_client_operation(*user_operation)
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
                    info!(
                        "Received host call: method_name='{}', call_id={}",
                        host_call.method_name, host_call.call_id
                    );

                    let method = host_call.get_param().map_err(|e| {
                        error!("Failed to parse host call parameters: {:#}", e);
                        e
                    })?;

                    info!("Processing host call method: {:?}", method);

                    // Handle the host call and create a response
                    use pwsh_core::host::{HostCallMethodReturn, RawUIMethodReturn};

                    let response = match method {
                        // For GetBufferSize, return a default console buffer size
                        pwsh_core::host::HostCallMethodWithParams::RawUIMethod(
                            pwsh_core::host::RawUIMethodParams::GetBufferSize,
                        ) => {
                            info!("Handling GetBufferSize - returning default console size");
                            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferSize(
                                120, 30,
                            ))
                        }

                        // For WriteProgress, just acknowledge (void return)
                        pwsh_core::host::HostCallMethodWithParams::UIMethod(
                            pwsh_core::host::UIMethodParams::WriteProgress(source_id, record),
                        ) => {
                            info!(
                                "Handling WriteProgress - source_id={}, record={}",
                                source_id, record
                            );
                            HostCallMethodReturn::UIMethod(
                                pwsh_core::host::UIMethodReturn::WriteProgress,
                            )
                        }

                        // For other methods, return not implemented error for now
                        other => {
                            warn!("Host call method not implemented: {:?}", other);
                            HostCallMethodReturn::Error(pwsh_core::host::HostError::NotImplemented)
                        }
                    };

                    // Submit the response
                    let host_response = host_call.submit_result(response);
                    info!(
                        "Created host call response for call_id={}",
                        host_response.call_id
                    );

                    // For now, we're not sending the response back yet - that requires more infrastructure
                    // TODO: Implement sending host call responses back to the server
                }
                ActiveSessionOutput::OperationSuccess => {
                    info!("Operation completed successfully");
                }
                ActiveSessionOutput::PipelineOutput {
                    output,
                    handle: _handle,
                } => match format_pipeline_output(&output) {
                    Ok(formatted) => {
                        info!("Pipeline output: {}", formatted);
                        println!("Pipeline output: {}", formatted);
                    }
                    Err(e) => {
                        warn!("Failed to format pipeline output: {}", e);
                        info!("Pipeline output (raw): {}", output);
                        println!("Pipeline output (raw): {}", output);
                    }
                },
            }
        }
    }
}

fn format_pipeline_output(output: &PipelineOutput) -> Result<String, anyhow::Error> {
    let Some(output_str) = output.data.as_string() else {
        return Err(anyhow::anyhow!("Pipeline output is not a string"));
    };

    decode_escaped_ps_string(&output_str)
}

/// Decode PowerShell Remoting Protocol escape sequences, like _x000A_
/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/301404a9-232f-439c-8644-1a213675bfac
fn decode_escaped_ps_string(input: &str) -> Result<String, anyhow::Error> {
    if input.is_empty() {
        return Ok(String::new());
    }

    // Split with capturing parentheses to include the separator in the resulting array
    let regex =
        Regex::new(r"(_x[0-9A-F]{4}_)").map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
    let parts: Vec<&str> = regex.split(input).collect();

    if parts.len() <= 1 {
        return Ok(input.to_string());
    }

    let mut result = String::new();
    let mut high_surrogate: Option<u16> = None;

    // We need to manually handle the split parts and captures
    let mut current_pos = 0;
    for captures in regex.find_iter(input) {
        // Add the text before the match
        if captures.start() > current_pos {
            result.push_str(&input[current_pos..captures.start()]);
            high_surrogate = None;
        }

        // Process the escaped sequence
        let escaped = captures.as_str();
        if let Some(hex_str) = escaped.strip_prefix("_x").and_then(|s| s.strip_suffix("_")) {
            match u16::from_str_radix(hex_str, 16) {
                Ok(code_unit) => {
                    if let Some(high) = high_surrogate {
                        // We have a high surrogate from before, try to form a surrogate pair
                        if (0xDC00..=0xDFFF).contains(&code_unit) {
                            // This is a low surrogate, form the pair
                            let code_point = 0x10000
                                + ((high as u32 - 0xD800) << 10)
                                + (code_unit as u32 - 0xDC00);
                            if let Some(ch) = char::from_u32(code_point) {
                                result.push(ch);
                            } else {
                                // Invalid code point, add the escaped sequence as-is
                                result.push_str(escaped);
                            }
                            high_surrogate = None;
                        } else {
                            // Not a low surrogate, add the previous high surrogate as-is and process this one
                            result.push_str("_x");
                            result.push_str(&format!("{:04X}", high));
                            result.push('_');

                            if (0xD800..=0xDBFF).contains(&code_unit) {
                                high_surrogate = Some(code_unit);
                            } else {
                                if let Some(ch) = char::from_u32(code_unit as u32) {
                                    result.push(ch);
                                } else {
                                    result.push_str(escaped);
                                }
                                high_surrogate = None;
                            }
                        }
                    } else if (0xD800..=0xDBFF).contains(&code_unit) {
                        // High surrogate, save it for the next iteration
                        high_surrogate = Some(code_unit);
                    } else {
                        // Regular character or low surrogate without high surrogate
                        if let Some(ch) = char::from_u32(code_unit as u32) {
                            result.push(ch);
                        } else {
                            // Invalid character, add the escaped sequence as-is
                            result.push_str(escaped);
                        }
                        high_surrogate = None;
                    }
                }
                Err(_) => {
                    // Invalid hex, add the escaped sequence as-is
                    result.push_str(escaped);
                    high_surrogate = None;
                }
            }
        } else {
            // Not a valid escape sequence, add as-is
            result.push_str(escaped);
            high_surrogate = None;
        }

        current_pos = captures.end();
    }

    // Add any remaining text after the last match
    if current_pos < input.len() {
        result.push_str(&input[current_pos..]);
    }

    // If we have an unmatched high surrogate at the end, add it as-is
    if let Some(high) = high_surrogate {
        result.push_str("_x");
        result.push_str(&format!("{:04X}", high));
        result.push('_');
    }

    Ok(result)
}
