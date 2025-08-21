mod config;
mod connection;
mod http_client;
mod network;
mod types;
mod user_input;

use anyhow::Context;
use clap::Parser;
use protocol_powershell_remoting::PipelineOutput;
use pwsh_core::connector::active_session::UserEvent;
use pwsh_core::connector::ActiveSessionOutput;
use regex::Regex;
use std::sync::mpsc;
use std::thread;
use tracing::{error, info, instrument, warn};

use config::{create_connector_config, init_logging, Args};
use connection::establish_connection;
use network::NetworkHandler;
use types::NextStep;
use user_input::UserInputHandler;

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging with the specified verbosity level
    init_logging(args.verbose)?;
    let _span = tracing::span!(tracing::Level::INFO, "main").entered();
    info!("Starting WinRM PowerShell client (Synchronous)");

    // Display connection information
    info!(
        "Connecting to {}:{} with user '{}' using {}",
        args.server,
        args.port,
        args.username,
        if args.https { "HTTPS" } else { "HTTP" }
    );

    // Create configuration and establish connection
    let config = create_connector_config(&args);
    let (active_session, next_request) = establish_connection(config)?;
    info!("Runspace pool is now open and ready for operations!");

    // Set up communication channels
    let (network_request_tx, network_request_rx) = mpsc::channel();
    let (network_response_tx, network_response_rx) = mpsc::channel();
    let (user_request_tx, user_request_rx) = mpsc::channel();
    let (user_event_tx, user_event_rx) = mpsc::channel();

    // Spawn network handler
    let mut network_handler = NetworkHandler::new(network_request_rx, network_response_tx);
    let network_handle = thread::spawn(move || {
        network_handler.run();
    });

    // Spawn user input/UI handler
    let mut user_input_handler = UserInputHandler::new(user_request_tx.clone(), user_event_rx);
    let user_handle = thread::spawn(move || {
        user_input_handler.run();
    });

    // Send initial network request
    network_request_tx
        .send(next_request)
        .context("Failed to send initial request")?;

    // Run the main event loop
    run_event_loop(
        active_session,
        network_response_rx,
        user_request_rx,
        network_request_tx,
        user_event_tx,
    )
    .inspect_err(|e| error!("Error in main event loop: {}", e))?;

    info!("Exiting main function");
    // Clean up threads (they will exit when channels are dropped)
    drop(network_handle);
    drop(user_handle);
    drop(_span);
    Ok(())
}

/// Main event loop that processes network responses and user requests
#[instrument(skip_all)]
fn run_event_loop(
    mut active_session: pwsh_core::connector::active_session::ActiveSession,
    network_response_rx: mpsc::Receiver<pwsh_core::connector::http::HttpResponse<String>>,
    user_request_rx: mpsc::Receiver<pwsh_core::connector::UserOperation>,
    network_request_tx: mpsc::Sender<pwsh_core::connector::http::HttpRequest<String>>,
    user_event_tx: mpsc::Sender<UserEvent>,
) -> anyhow::Result<()> {
    loop {
        // Use select! equivalent for synchronous channels
        let next_step = select_sync(&network_response_rx, &user_request_rx)?;

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

                vec![active_session
                    .accept_client_operation(user_operation)
                    .map_err(|e| {
                        error!("Failed to accept user operation: {:#}", e);
                        e
                    })
                    .context("Failed to accept user operation")?]
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
                            .context("Failed to send HTTP request")?;
                    }
                }
                ActiveSessionOutput::SendBackError(e) => {
                    error!("Error in session step: {}", e);
                    return Err(anyhow::anyhow!("Session step failed: {}", e));
                }
                ActiveSessionOutput::UserEvent(event) => {
                    // Send all user events to the UI thread
                    if let Err(e) = user_event_tx.send(event) {
                        error!("Failed to send user event: {}", e);
                    }
                }
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

/// Synchronous select equivalent for two receivers
fn select_sync(
    network_rx: &mpsc::Receiver<pwsh_core::connector::http::HttpResponse<String>>,
    user_rx: &mpsc::Receiver<pwsh_core::connector::UserOperation>,
) -> anyhow::Result<NextStep> {
    use std::sync::mpsc::TryRecvError;

    loop {
        // Try to receive from network first
        match network_rx.try_recv() {
            Ok(response) => return Ok(NextStep::NetworkResponse(response)),
            Err(TryRecvError::Empty) => {
                // Try user channel
                match user_rx.try_recv() {
                    Ok(request) => return Ok(NextStep::UserRequest(request)),
                    Err(TryRecvError::Empty) => {
                        // Both channels empty, wait a bit and try again
                        thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(TryRecvError::Disconnected) => {
                        return Err(anyhow::anyhow!("User request channel disconnected"));
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                return Err(anyhow::anyhow!("Network response channel disconnected"));
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
