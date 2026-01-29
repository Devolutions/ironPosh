use futures::StreamExt;
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_terminal::Terminal;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, error, info, warn};

use crate::types::TerminalOperation;

#[derive(Debug)]
enum UserInput {
    Cmd(String),
    Interrupt,
    Eof,
}

fn sanitize_prompt(mut prompt: String) -> String {
    // Some prompts may contain newlines; keep only the last line for a single-line UI prompt.
    if prompt.contains('\n') || prompt.contains('\r') {
        prompt = prompt.lines().last().unwrap_or("> ").to_string();
    }

    if prompt.is_empty() {
        return "> ".to_string();
    }

    prompt
}

async fn fetch_remote_prompt(client: &mut RemoteAsyncPowershellClient) -> String {
    // Use PowerShell's `prompt` function so user customizations ($PROFILE, etc.) are reflected.
    let mut stream = match client.send_script("prompt".to_string()).await {
        Ok(stream) => stream.boxed(),
        Err(e) => {
            warn!(error = %e, "failed to request remote prompt; falling back");
            return "> ".to_string();
        }
    };

    let mut last_prompt: Option<String> = None;

    while let Some(ev) = stream.next().await {
        match ev {
            UserEvent::PipelineOutput { output, .. } => {
                match output.format_as_displyable_string() {
                    Ok(text) => {
                        if !text.is_empty() {
                            last_prompt = Some(text);
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to parse remote prompt output");
                    }
                }
            }
            UserEvent::ErrorRecord { error_record, .. } => {
                warn!(error = %error_record.render_concise(), "remote prompt command returned an error");
            }
            UserEvent::PipelineFinished { .. } => break,
            UserEvent::PipelineCreated { .. } => {}
        }
    }

    let prompt = sanitize_prompt(last_prompt.unwrap_or_else(|| "> ".to_string()));
    debug!(prompt = %prompt, "remote prompt fetched");
    prompt
}

async fn request_prompt(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: &Sender<TerminalOperation>,
) {
    let prompt = fetch_remote_prompt(client).await;
    let _ = terminal_op_tx
        .send(TerminalOperation::RequestInput { prompt })
        .await;
}

/// Run the UI thread that owns the terminal and processes UI operations
fn run_ui_thread(
    mut terminal: Terminal,
    mut terminal_op_rx: Receiver<TerminalOperation>,
    user_input_tx: Sender<UserInput>,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use ironposh_terminal::ReadOutcome;
        use std::io::Write;

        info!("UI thread starting with unified queue");
        let mut io = terminal.stdio();

        let _ui = tracing::span!(tracing::Level::INFO, "UI Thread").entered();
        // Drain all pending UI ops
        while let Some(op) = terminal_op_rx.blocking_recv() {
            info!(op = ?op, "Processing terminal operation");
            match op {
                TerminalOperation::Apply(ops) => {
                    debug!(count = ops.len(), "applying terminal operations");
                    for o in ops {
                        io.apply_op(o);
                    }
                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal");
                        return Err(e);
                    }
                }
                TerminalOperation::Print(s) => {
                    debug!(chars = s.len(), "printing output");
                    if let Err(e) = writeln!(io, "{s}") {
                        error!(error = %e, "failed to write to terminal");
                        return Err(e.into());
                    }
                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal after print");
                        return Err(e);
                    }
                }
                TerminalOperation::Write { text, newline } => {
                    debug!(chars = text.len(), newline, "writing output");
                    if newline {
                        if let Err(e) = writeln!(io, "{text}") {
                            error!(error = %e, "failed to write line to terminal");
                            return Err(e.into());
                        }
                    } else if let Err(e) = write!(io, "{text}") {
                        error!(error = %e, "failed to write to terminal");
                        return Err(e.into());
                    }

                    if let Err(e) = io.render() {
                        error!(error = %e, "failed to render terminal after write");
                        return Err(e);
                    }
                }
                TerminalOperation::SetWindowTitle { title } => {
                    debug!(title = %title, "setting host window title");
                    // Best-effort: write directly to the host terminal using Crossterm.
                    // Do not route through the guest terminal emulator.
                    if let Err(e) =
                        crossterm::execute!(std::io::stdout(), crossterm::terminal::SetTitle(title))
                    {
                        warn!(error = %e, "failed to set window title");
                    }
                }
                TerminalOperation::RequestInput { prompt } => {
                    debug!(prompt = %prompt, "reading user input");
                    match io.read_line(&prompt) {
                        Ok(ReadOutcome::Line(s)) => {
                            info!(command = %s.trim(), "user entered command");
                            if user_input_tx.blocking_send(UserInput::Cmd(s)).is_err() {
                                warn!("failed to send command to REPL - channel closed");
                                return Ok(());
                            }
                        }
                        Ok(ReadOutcome::Interrupt) => {
                            info!("user pressed Ctrl+C");
                            if user_input_tx.blocking_send(UserInput::Interrupt).is_err() {
                                warn!("failed to send interrupt to REPL - channel closed");
                                return Ok(());
                            }
                        }
                        Ok(ReadOutcome::Eof) => {
                            info!("received EOF from user input");
                            let _ = user_input_tx.blocking_send(UserInput::Eof);
                            return Ok(());
                        }
                        Err(e) => {
                            error!(error = %e, "error reading user input");
                            return Err(e.into());
                        }
                    }
                }
                TerminalOperation::CheckInterrupt => {
                    if let Some(read_line) = io.try_read_line()? {
                        match read_line {
                            ReadOutcome::Line(s) => {
                                info!(command = %s.trim(), "user entered command");
                                if user_input_tx.blocking_send(UserInput::Cmd(s)).is_err() {
                                    warn!("failed to send command to REPL - channel closed");
                                    return Ok(());
                                }
                            }
                            ReadOutcome::Interrupt => {
                                info!("user pressed Ctrl+C");
                                if user_input_tx.blocking_send(UserInput::Interrupt).is_err() {
                                    warn!("failed to send interrupt to REPL - channel closed");
                                    return Ok(());
                                }
                            }
                            ReadOutcome::Eof => {
                                info!("received EOF from user input");
                                let _ = user_input_tx.blocking_send(UserInput::Eof);
                                return Ok(());
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(())
    })
}

/// Run the main REPL event loop
async fn run_repl_loop(
    client: &mut RemoteAsyncPowershellClient,
    terminal_op_tx: Sender<TerminalOperation>,
    mut user_input_rx: Receiver<UserInput>,
) -> anyhow::Result<()> {
    info!("Starting unified REPL loop");

    // Ask for the first prompt
    request_prompt(client, &terminal_op_tx).await;

    // Async REPL loop
    let mut current_pipeline = None;
    let mut current_stream = None::<futures::stream::BoxStream<'_, UserEvent>>;

    loop {
        tokio::select! {
            // User input from UI thread
            Some(msg) = user_input_rx.recv() => {
                debug!(?msg, "Received message from UI thread");
                match msg {
                    UserInput::Eof => {
                        info!("Received EOF, exiting REPL");
                        break;
                    }
                    UserInput::Interrupt => {
                        if let Some(h) = current_pipeline.take() {
                            info!(pipeline = ?h, "Killing active pipeline due to interrupt");
                            client.kill_pipeline(h).await?;
                            current_stream = None;
                        }
                        request_prompt(client, &terminal_op_tx).await;
                    }
                    UserInput::Cmd(cmd) => {
                        let cmd = cmd.trim().to_string();
                        info!(command = %cmd, "processing command");

                        if cmd.eq_ignore_ascii_case("exit") {
                            info!("Exit command received, terminating REPL");
                            break;
                        }

                        if cmd.is_empty() {
                            debug!("Empty command, requesting new prompt");
                            request_prompt(client, &terminal_op_tx).await;
                            continue;
                        }

                        // Start a pipeline
                        info!(command = %cmd, "Sending command to PowerShell");
                        match client.send_script(cmd).await {
                            Ok(stream) => {
                                info!("Command sent successfully, waiting for events");
                                current_stream = Some(stream.boxed());
                                current_pipeline = None; // will be set on PipelineCreated
                            }
                            Err(e) => {
                                error!("Failed to send command: {}", e);
                                let _ = terminal_op_tx.send(TerminalOperation::Print(format!("Error sending command: {e}"))).await;
                                request_prompt(client, &terminal_op_tx).await;
                            }
                        }
                    }
                }
            }

            // Pipeline events
            Some(ev) = async {
                match &mut current_stream {
                    Some(s) => s.next().await,
                    None => futures::future::pending().await,
                }
            } => {
                debug!(?ev,"Received pipeline event");
                let _ = terminal_op_tx.send(TerminalOperation::CheckInterrupt).await;
                match ev {
                    UserEvent::PipelineCreated { pipeline } => {
                        info!(pipeline = ?pipeline, "Pipeline created");
                        current_pipeline = Some(pipeline);
                    }
                    UserEvent::PipelineFinished { .. } => {
                        info!("Pipeline finished");
                        current_pipeline = None;
                        current_stream = None;
                        // Request new prompt after pipeline finishes
                        request_prompt(client, &terminal_op_tx).await;
                    }
                    UserEvent::PipelineOutput { output, .. } => {
                        debug!("Received pipeline output");
                        let text = match output.format_as_displyable_string() {
                            Ok(s) => {
                                debug!("Formatted output: {} chars", s.len());
                                s
                            }
                            Err(e) => {
                                error!("Error formatting output: {}", e);
                                format!("Error formatting output: {e}")
                            }
                        };
                        let _ = terminal_op_tx.send(TerminalOperation::Print(text)).await;
                    }
                    UserEvent::ErrorRecord { error_record, .. } => {
                        debug!("Received error record");
                        let error_text = error_record.render_concise();
                        let _ = terminal_op_tx.send(TerminalOperation::Print(format!("Error: {error_text}"))).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run simple REPL mode using basic stdin/stdout
pub async fn run_simple_repl(
    client: &mut RemoteAsyncPowershellClient,
    terminal: Terminal,
    mut hostcall_ui_rx: tokio::sync::mpsc::Receiver<TerminalOperation>,
) -> anyhow::Result<()> {
    info!("Starting async REPL with unified UI queue");

    // Channels: UI thread -> REPL (user input events)
    let (terminal_request_tx, terminal_request_rx) = tokio::sync::mpsc::channel::<UserInput>(32);
    let (terminal_op_tx, terminal_op_rx) = tokio::sync::mpsc::channel::<TerminalOperation>(32);

    let terminal_op_tx_1 = terminal_op_tx.clone();
    let forward_handle = tokio::spawn(async move {
        while let Some(op) = hostcall_ui_rx.recv().await {
            if terminal_op_tx_1.send(op).await.is_err() {
                warn!("UI operation channel closed, stopping forwarder");
                break;
            }
        }
    });

    info!("Created unified communication channels");
    let ui_handle = run_ui_thread(terminal, terminal_op_rx, terminal_request_tx.clone());

    info!("UI thread and forwarder tasks spawned, starting unified REPL loop");
    let repl_result = run_repl_loop(client, terminal_op_tx, terminal_request_rx).await;

    info!("REPL loop ending, cleaning up tasks");

    ui_handle.abort();
    forward_handle.abort();

    info!("Unified async REPL completed");
    repl_result
}
