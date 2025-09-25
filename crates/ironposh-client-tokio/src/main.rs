mod config;
mod hostcall;
mod http_client;
mod repl;
mod types;

use clap::Parser;
use futures::StreamExt;
use ironposh_client_async::{HostResponse, RemoteAsyncPowershellClient};
use ironposh_terminal::Terminal;
use tracing::{error, info, instrument, warn};

use config::{create_connector_config, init_logging, Args};
use http_client::ReqwestHttpClient;
use types::TerminalOperation;

#[tokio::main]
#[instrument(name = "main", level = "info")]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging with the specified verbosity level
    init_logging(args.verbose)?;
    info!("Starting WinRM PowerShell client (Async/Tokio)");

    // Display connection information
    info!(
        server = %args.server,
        port = args.port,
        username = %args.username,
        scheme = %if args.https { "HTTPS" } else { "HTTP" },
        "connecting to server"
    );

    // Create terminal early to get real dimensions for PowerShell host info
    let mut terminal = Terminal::new(2000)?;
    let (cols, rows) = terminal.size()?;
    info!("Terminal created with size: {}x{}", cols, rows);

    // Create configuration and HTTP client with real terminal dimensions
    let config = create_connector_config(&args, cols, rows)?;
    let http_client = ReqwestHttpClient::new();

    // Create the PowerShell client
    let (client, connection_task) = RemoteAsyncPowershellClient::open_task(config, http_client);

    // Extract host I/O for handling host calls
    let (mut client, host_io) = client.take_host_io();
    let (host_call_rx, submitter) = host_io.into_parts();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::channel(100); // For future UI integration

    // Spawn host call handler task
    let host_call_handle =
        tokio::spawn(hostcall::handle_host_calls(host_call_rx, submitter, ui_tx));

    info!("Runspace pool is now open and ready for operations!");

    // Check if we have a command to execute
    if let Some(command) = args.command {
        // Non-interactive mode: execute command and exit
        info!(command = %command, "executing command in non-interactive mode");

        // Spawn connection task
        let connection_handle = tokio::spawn(connection_task);

        // Execute command
        let mut stream = client.send_command(command).await?;

        while let Some(event) = stream.next().await {
            unimplemented!("{event:?}");
        }
        // Clean up
        connection_handle.abort();
        host_call_handle.abort();
    } else {
        // Interactive mode: simple REPL
        info!("starting simple interactive mode");

        // Spawn connection task
        let _connection_handle = tokio::spawn(connection_task);
        let _host_call_handle = host_call_handle;

        if let Err(e) = repl::run_simple_repl(&mut client, terminal, ui_rx).await {
            error!(error = %e, "Interactive mode failed");
            eprintln!("Interactive mode failed: {e}");
            std::process::exit(1);
        }
    }

    info!("Exiting main function");
    Ok(())
}
