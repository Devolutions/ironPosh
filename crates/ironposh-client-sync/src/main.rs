mod config;
mod connection;
mod http_client;
mod kerberos;
mod network;
mod types;
mod user_input;

use anyhow::Context;
use clap::Parser;
use ironposh_client_core::connector::http::HttpResponseTargeted;
use ironposh_client_core::connector::ActiveSessionOutput;
use ironposh_client_core::connector::{active_session::UserEvent, conntion_pool::TrySend};
use ironposh_client_core::host::HostCall;
use std::sync::mpsc;
use std::thread;
use tracing::{debug, error, info, instrument, warn};

use config::{create_connector_config, init_logging, Args};
use connection::RemotePowershell;
use http_client::UreqHttpClient;
use network::NetworkHandler;
use types::NextStep;
use user_input::UserInputHandler;

/// Establish connection to the PowerShell remote server
fn establish_connection(
    config: ironposh_client_core::connector::WinRmConfig,
) -> anyhow::Result<(
    ironposh_client_core::connector::active_session::ActiveSession,
    TrySend,
    UreqHttpClient,
)> {
    let mut client = UreqHttpClient::new();
    let remote_ps = RemotePowershell::open(config, &mut client)?;
    let (active_session, next_request) = remote_ps.into_components();
    Ok((active_session, next_request, client))
}

#[instrument(name = "main", level = "info")]
fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging. If it fails, we can't log, so just print and exit.
    if let Err(e) = init_logging(args.verbose) {
        eprintln!("Failed to initialize logging: {e}");
        // Exit with a non-zero status code to indicate failure
        std::process::exit(1);
    }

    // Run the actual application logic and handle any errors
    if let Err(e) = run_app(&args) {
        // Log the error before exiting. This is crucial.
        error!("Application failed to run: {:?}", e);

        // The program will now exit, and the log buffer should be flushed upon exit.
        return Err(e);
    }

    Ok(())
}

/// The main application logic, extracted to a separate function.
fn run_app(args: &Args) -> anyhow::Result<()> {
    info!("Starting WinRM PowerShell client (Synchronous)");

    // Display connection information
    info!(
        server = %args.server,
        port = args.port,
        username = %args.username,
        scheme = %if args.https { "HTTPS" } else { "HTTP" },
        "connecting to server"
    );

    // Create configuration and establish connection
    let config = create_connector_config(args)?;
    let (active_session, next_request, http_client) = establish_connection(config)?;
    info!("Runspace pool is now open and ready for operations!");

    // Set up communication channels
    let (network_request_tx, network_request_rx) = mpsc::channel();
    let (network_response_tx, network_response_rx) = mpsc::channel();
    let (user_request_tx, user_request_rx) = mpsc::channel();
    let (user_event_tx, user_event_rx) = mpsc::channel();

    // Spawn network handler
    let mut network_handler =
        NetworkHandler::new(network_request_rx, network_response_tx, http_client);
    let network_handle = thread::spawn(move || {
        network_handler.run();
    });

    // Spawn user input/UI handler
    let mut user_input_handler = UserInputHandler::new(user_request_tx.clone(), user_event_rx);
    let user_handle = thread::spawn(move || {
        let _ = user_input_handler
            .run()
            .inspect_err(|e| error!(err = ?e, "User input handler failed"));
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
    Ok(())
}

/// Main event loop that processes network responses and user requests
#[instrument(level = "info", skip_all, fields(iterations = 0u64))]
fn run_event_loop(
    mut active_session: ironposh_client_core::connector::active_session::ActiveSession,
    network_response_rx: mpsc::Receiver<HttpResponseTargeted>,
    user_request_rx: mpsc::Receiver<ironposh_client_core::connector::UserOperation>,
    network_request_tx: mpsc::Sender<TrySend>,
    user_event_tx: mpsc::Sender<UserEvent>,
) -> anyhow::Result<()> {
    // Clone the network_request_tx for host call responses
    let network_tx_for_host_calls = network_request_tx.clone();

    loop {
        // Use select! equivalent for synchronous channels
        let next_step = select_sync(&network_response_rx, &user_request_rx)?;

        info!(next_step = %next_step, "processing step");

        let step_results = match next_step {
            NextStep::NetworkResponse(http_response) => {
                info!(
                    target: "network",
                    body_length = http_response.response().body.len(),
                    "processing network response"
                );

                active_session
                    .accept_server_response(http_response)
                    .map_err(|e| {
                        error!(target: "network", error = %e, "failed to accept server response");
                        e
                    })
                    .context("Failed to accept server response")?
            }
            NextStep::UserRequest(user_operation) => {
                info!(target: "user", operation = ?user_operation, "processing user operation");

                vec![active_session
                    .accept_client_operation(*user_operation)
                    .map_err(|e| {
                        error!(target: "user", error = %e, "failed to accept user operation");
                        e
                    })
                    .context("Failed to accept user operation")?]
            }
        };

        info!(
            step_result_count = step_results.len(),
            "received server response, processing step results"
        );

        for step_result in step_results {
            info!(step_result = ?step_result, "processing step result");
            match step_result {
                ActiveSessionOutput::SendBack(http_requests) => {
                    info!(
                        target: "network",
                        request_count = http_requests.len(),
                        "sending HTTP requests"
                    );
                    for http_request in http_requests {
                        network_request_tx
                            .send(http_request)
                            .context("Failed to send HTTP request")?;
                    }
                }
                ActiveSessionOutput::SendBackError(e) => {
                    error!(target: "session", error = %e, "session step failed");
                    return Err(anyhow::anyhow!("Session step failed: {}", e));
                }
                ActiveSessionOutput::UserEvent(event) => {
                    info!(target: "user", event = ?event, "sending user event");
                    // Send all user events to the UI thread
                    if let Err(e) = user_event_tx.send(event) {
                        error!(target: "user", error = %e, "failed to send user event");
                    }
                }
                ActiveSessionOutput::HostCall(host_call) => {
                    let scope = { host_call.scope() };
                    let call_id = host_call.call_id();
                    info!("=== HOST CALL === Method: {} ID: {} Scope: {:?}", host_call.method_name(), call_id, scope);
                    let submission = match host_call {
                        HostCall::GetName { transport } => {
                            // Extract parameters and get the result transport
                            let (_params, result_transport) = transport.into_parts();
                            let host_name = "PowerShell-Host".to_string(); // In real implementation, get actual host name

                            result_transport.accept_result(host_name)
                        }
                        HostCall::SetCursorPosition { transport } => {
                            let (params, result_transport) = transport.into_parts();

                            let coordinates = params.0; // Access first element of tuple
                            info!("SetCursorPosition: x:{}, y:{}", coordinates.x, coordinates.y);

                            // Use crossterm to actually set the cursor position
                            use crossterm::{cursor, ExecutableCommand};
                            use std::io::stdout;

                            // Bounds check coordinates to prevent overflow
                            let safe_x = coordinates.x.max(0).min(u16::MAX as i32) as u16;
                            let safe_y = coordinates.y.max(0).min(u16::MAX as i32) as u16;

                            if coordinates.x != safe_x as i32 || coordinates.y != safe_y as i32 {
                                warn!(
                                    "Coordinates ({}, {}) were clamped to ({}, {}) to prevent overflow",
                                    coordinates.x, coordinates.y, safe_x, safe_y
                                );
                            }

                            let mut stdout = stdout();
                            if let Err(e) = stdout.execute(cursor::MoveTo(safe_x, safe_y)) {
                                warn!("Failed to set cursor position: {}", e);
                            } else {
                                debug!("Successfully moved cursor to ({}, {})", safe_x, safe_y);
                            }

                            result_transport.accept_result(())
                        }
                        HostCall::SetBufferContents1 { transport } => {
                            let (params, result_transport) = transport.into_parts();

                            let rectangle = params.0;
                            let buffer_cell = params.1;
                            info!("SetBufferContents1: rectangle={{left:{}, top:{}, right:{}, bottom:{}}}, cell={{char:'{}', fg:{}, bg:{}}}",
                                rectangle.left, rectangle.top, rectangle.right, rectangle.bottom,
                                buffer_cell.character, buffer_cell.foreground, buffer_cell.background);

                            use crossterm::{cursor, style, terminal, QueueableCommand};
                            use std::io::{stdout, Write};

                            let mut stdout = stdout();

                            // Check if this is a screen clear operation
                            // PowerShell sends clear as a rectangle covering the entire screen with space characters
                            let is_screen_clear = buffer_cell.character == ' ' &&
                                rectangle.left == 0 &&
                                rectangle.top == 0;

                            if is_screen_clear {
                                // This is likely a screen clear operation - use proper clear command
                                if let Err(e) = stdout.queue(terminal::Clear(terminal::ClearType::All)) {
                                    warn!("Failed to clear screen: {}", e);
                                } else if let Err(e) = stdout.queue(cursor::MoveTo(0, 0)) {
                                    warn!("Failed to move cursor to home position: {}", e);
                                }

                                if let Err(e) = stdout.flush() {
                                    warn!("Failed to flush clear command: {}", e);
                                } else {
                                    debug!("Successfully cleared screen");
                                }
                            } else {
                                // Regular buffer content setting - convert PS colors to crossterm colors
                                let fg_color = match buffer_cell.foreground {
                                    0 => style::Color::Black,
                                    1 => style::Color::DarkBlue,
                                    2 => style::Color::DarkGreen,
                                    3 => style::Color::DarkCyan,
                                    4 => style::Color::DarkRed,
                                    5 => style::Color::DarkMagenta,
                                    6 => style::Color::DarkYellow,
                                    7 => style::Color::Grey,
                                    8 => style::Color::DarkGrey,
                                    9 => style::Color::Blue,
                                    10 => style::Color::Green,
                                    11 => style::Color::Cyan,
                                    12 => style::Color::Red,
                                    13 => style::Color::Magenta,
                                    14 => style::Color::Yellow,
                                    15 => style::Color::White,
                                    _ => style::Color::White,
                                };

                                let bg_color = match buffer_cell.background {
                                    0 => style::Color::Black,
                                    1 => style::Color::DarkBlue,
                                    2 => style::Color::DarkGreen,
                                    3 => style::Color::DarkCyan,
                                    4 => style::Color::DarkRed,
                                    5 => style::Color::DarkMagenta,
                                    6 => style::Color::DarkYellow,
                                    7 => style::Color::Grey,
                                    8 => style::Color::DarkGrey,
                                    9 => style::Color::Blue,
                                    10 => style::Color::Green,
                                    11 => style::Color::Cyan,
                                    12 => style::Color::Red,
                                    13 => style::Color::Magenta,
                                    14 => style::Color::Yellow,
                                    15 => style::Color::White,
                                    _ => style::Color::Black,
                                };

                                // Fill the rectangle area with the character and colors
                                for y in rectangle.top..=rectangle.bottom {
                                    let safe_x = rectangle.left.max(0).min(u16::MAX as i32) as u16;
                                    let safe_y = y.max(0).min(u16::MAX as i32) as u16;

                                    if let Err(e) = stdout.queue(cursor::MoveTo(safe_x, safe_y)) {
                                        warn!("Failed to move cursor to ({}, {}): {}", safe_x, safe_y, e);
                                        continue;
                                    }

                                    let width = (rectangle.right - rectangle.left + 1) as usize;
                                    let fill_string = buffer_cell.character.to_string().repeat(width);

                                    if stdout.queue(style::SetForegroundColor(fg_color)).is_ok()
                                        && stdout.queue(style::SetBackgroundColor(bg_color)).is_ok()
                                        && stdout.queue(style::Print(fill_string)).is_ok()
                                    {
                                        // Operations succeeded
                                    } else {
                                        warn!("Failed to write buffer contents at line {}", y);
                                    }
                                }

                                if let Err(e) = stdout.queue(style::ResetColor) {
                                    warn!("Failed to reset colors: {}", e);
                                }
                                if let Err(e) = stdout.flush() {
                                    warn!("Failed to flush buffer contents: {}", e);
                                } else {
                                    debug!("Successfully set buffer contents for rectangle {:?}", rectangle);
                                }
                            }

                            result_transport.accept_result(())
                        }
                        _ => {
                            warn!("Unhandled host call type: {}", host_call.method_name());
                            todo!("Handle other host call types")
                        }
                    };

                    active_session
                        .accept_client_operation(
                            ironposh_client_core::connector::UserOperation::SubmitHostResponse {
                                call_id,
                                scope,
                                submission,
                            },
                        )
                        .context("Failed to send host call response to active session")?;
                }
                ActiveSessionOutput::OperationSuccess => {
                    info!(target: "session", "operation completed successfully");
                }
            }
        }
    }
}

/// Synchronous select equivalent for two receivers
fn select_sync(
    network_rx: &mpsc::Receiver<HttpResponseTargeted>,
    user_rx: &mpsc::Receiver<ironposh_client_core::connector::UserOperation>,
) -> anyhow::Result<NextStep> {
    use std::sync::mpsc::TryRecvError;

    loop {
        // Try to receive from network first
        match network_rx.try_recv() {
            Ok(response) => return Ok(NextStep::NetworkResponse(response)),
            Err(TryRecvError::Empty) => {
                // Try user channel
                match user_rx.try_recv() {
                    Ok(request) => return Ok(NextStep::UserRequest(Box::new(request))),
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
