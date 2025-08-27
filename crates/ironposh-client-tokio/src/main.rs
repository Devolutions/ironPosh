mod config;
mod http_client;

use anyhow::Context;
use clap::Parser;
use ironposh_client_async::remote_client::RemoteAsyncPowershellClient;
use tracing::{error, info, instrument};

use config::{create_connector_config, init_logging, Args};
use http_client::ReqwestHttpClient;

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

    // Create configuration and HTTP client
    let config = create_connector_config(&args);
    let http_client = ReqwestHttpClient::new();

    // Create the PowerShell client
    let (mut client, connection_task) = RemoteAsyncPowershellClient::open_task(config, http_client);

    // Spawn the connection task (no tracing in spawned task to avoid Send issues)
    let connection_handle = tokio::spawn(async move {
        if let Err(e) = connection_task.await {
            error!(error = %e, "Connection task ended with error");
        }
    });

    info!("Runspace pool is now open and ready for operations!");

    // Check if we have a command to execute
    if let Some(command) = args.command {
        // Non-interactive mode: execute command and exit
        info!(command = %command, "executing command in non-interactive mode");

        match client.send_command(command.clone(), false).await {
            Ok(output) => {
                println!("{output}");
                info!("Command executed successfully");
            }
            Err(e) => {
                error!(error = %e, "Failed to execute command");
                eprintln!("Error executing command: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode: start REPL
        info!("starting interactive mode");
        if let Err(e) = run_interactive_mode(&mut client).await {
            error!(error = %e, "Interactive mode failed");
            eprintln!("Interactive mode failed: {e}");
            std::process::exit(1);
        }
    }

    // Clean up
    connection_handle.abort();
    info!("Exiting main function");
    Ok(())
}

/// Run interactive REPL mode
async fn run_interactive_mode(client: &mut RemoteAsyncPowershellClient) -> anyhow::Result<()> {
    use std::io::Write;
    use tokio::io::{self, AsyncBufReadExt, BufReader};

    println!("IronPosh Interactive PowerShell Client");
    println!("Enter PowerShell commands or 'exit' to quit");
    let prompt = client.prompt().await.unwrap_or_else(|_| "PS> ".to_string());
    print!("{prompt} ");
    std::io::stdout()
        .flush()
        .context("Failed to flush stdout")?;

    let stdin = io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines
        .next_line()
        .await
        .context("Failed to read from stdin")?
    {
        let command = line.trim();

        if command.is_empty() {
            let prompt = client.prompt().await.unwrap_or_else(|_| "PS> ".to_string());
            print!("{prompt} ");
            std::io::stdout()
                .flush()
                .context("Failed to flush stdout")?;
            continue;
        }

        if command.eq_ignore_ascii_case("exit") || command.eq_ignore_ascii_case("quit") {
            info!("User requested exit");
            break;
        }

        info!(command = %command, "executing user command");

        match client.send_command(command.to_string(), false).await {
            Ok(output) => {
                print!("{output}");
                if !output.ends_with('\n') {
                    println!();
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to execute command");
                eprintln!("Error: {e}");
            }
        }

        let prompt = client.prompt().await.unwrap_or_else(|_| "PS> ".to_string());
        print!("{prompt} ");
        std::io::stdout()
            .flush()
            .context("Failed to flush stdout")?;
    }

    Ok(())
}
