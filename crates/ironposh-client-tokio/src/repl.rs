use futures::StreamExt;
use ironposh_client_async::RemoteAsyncPowershellClient;
use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_terminal::{ReadOutcome, Terminal};
use std::io::Write;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::types::UiOp;

#[derive(Debug)]
enum UiToRepl {
    Cmd(String),
    Interrupt,
    Eof,
}

/// Run simple REPL mode using basic stdin/stdout
pub async fn run_simple_repl(
    client: &mut RemoteAsyncPowershellClient,
    terminal: Terminal,
) -> anyhow::Result<()> {
    info!("Starting async REPL");

    // Channels: UI thread -> REPL, and REPL -> UI thread
    let (ui_to_repl_tx, mut ui_to_repl_rx) = mpsc::unbounded_channel::<UiToRepl>();
    let (repl_to_ui_tx, repl_to_ui_rx) = mpsc::unbounded_channel::<UiOp>();

    info!("Created communication channels");
    let terminate_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ui_terminate_flag = std::sync::Arc::clone(&terminate_flag);

    // 1) UI thread owns the terminal (blocking)
    let ui_handle = tokio::task::spawn_blocking({
        let mut terminal = terminal;
        let mut repl_to_ui_rx = repl_to_ui_rx;
        move || -> anyhow::Result<()> {
            info!("UI thread starting");
            let mut io = terminal.stdio();

            while !ui_terminate_flag.load(std::sync::atomic::Ordering::Relaxed) {
                // First drain any UiOps the REPL sent (render on the same thread)
                let mut ui_ops_processed = 0;
                while let Ok(op) = repl_to_ui_rx.try_recv() {
                    ui_ops_processed += 1;
                    debug!("Processing UI operation: {:?}", op);
                    match op {
                        UiOp::Apply(ops) => {
                            debug!("Applying {} terminal operations", ops.len());
                            for o in ops {
                                io.apply_op(o);
                            }
                            if let Err(e) = io.render() {
                                error!("Failed to render terminal: {}", e);
                                return Err(e);
                            }
                        }
                        UiOp::Print(s) => {
                            debug!("Printing output: {} chars", s.len());
                            if let Err(e) = writeln!(io, "{s}") {
                                error!("Failed to write to terminal: {}", e);
                                return Err(e.into());
                            }
                            if let Err(e) = io.render() {
                                error!("Failed to render terminal after print: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
                if ui_ops_processed > 0 {
                    debug!("Processed {} UI operations", ui_ops_processed);
                }

                debug!("Reading user input...");
                match io.read_line("> ") {
                    Ok(ReadOutcome::Line(s)) => {
                        info!(command = %s.trim(), "User entered command");
                        if ui_to_repl_tx.send(UiToRepl::Cmd(s)).is_err() {
                            warn!("Failed to send command to REPL - channel closed");
                            break;
                        }
                    }
                    Ok(ReadOutcome::Interrupt) => {
                        info!("User pressed Ctrl+C");
                        if ui_to_repl_tx.send(UiToRepl::Interrupt).is_err() {
                            warn!("Failed to send interrupt to REPL - channel closed");
                            break;
                        }
                        // reprompt without printing a new line, mimic sync
                        continue;
                    }
                    Ok(ReadOutcome::Eof) => {
                        info!("Received EOF from user input");
                        let _ = ui_to_repl_tx.send(UiToRepl::Eof);
                        break;
                    }
                    Err(e) => {
                        error!("Error reading user input: {}", e);
                        return Err(e.into());
                    }
                }
            }
            info!("UI thread ending");
            Ok(())
        }
    });

    info!("UI thread spawned, starting async REPL loop");

    // 2) Async REPL loop
    let mut current_pipeline = None;
    let mut current_stream = None::<futures::stream::BoxStream<'_, UserEvent>>; // Stream<UserEvent>

    loop {
        tokio::select! {
            // a) User typed something
            Some(msg) = ui_to_repl_rx.recv() => {
                debug!(?msg, "Received message from UI thread");
                match msg {
                    UiToRepl::Eof => {
                        info!("Received EOF, exiting REPL");
                        break;
                    }
                    UiToRepl::Interrupt => {
                        if let Some(h) = current_pipeline.take() {
                            info!(pipeline = ?h, "Killing active pipeline due to interrupt");
                            client.kill_pipeline(h).await?;
                        } else {
                            debug!("Interrupt received but no active pipeline to kill");
                        }
                        // keep waiting; sync impl reprompts immediately
                    }
                    UiToRepl::Cmd(cmd) => {
                        let cmd = cmd.trim().to_string();
                        info!(command = %cmd, "Processing command");

                        if cmd.eq_ignore_ascii_case("exit") {
                            info!("Exit command received, terminating REPL");
                            break;
                        }
                        if cmd.is_empty() {
                            debug!("Empty command, ignoring");
                            continue;
                        }

                        // Start a pipeline: script + output stream
                        info!(command = %cmd, "Sending command to PowerShell");
                        match client.send_command(cmd).await {
                            Ok(stream) => {
                                info!("Command sent successfully, waiting for events");
                                current_stream = Some(stream.boxed());
                                current_pipeline = None; // will be set on PipelineCreated
                            }
                            Err(e) => {
                                error!("Failed to send command: {}", e);
                                let _ = repl_to_ui_tx.send(UiOp::Print(format!("Error sending command: {e}")));
                            }
                        }
                    }
                }
            }

            // b) Pipeline events
            Some(ev) = async {
                match &mut current_stream {
                    Some(s) => s.next().await,
                    None => futures::future::pending().await,
                }
            } => {
                debug!("Received pipeline event: {:?}", ev);
                match ev {
                    UserEvent::PipelineCreated { pipeline } => {
                        info!(pipeline = ?pipeline, "Pipeline created");
                        current_pipeline = Some(pipeline);
                    }
                    UserEvent::PipelineFinished { .. } => {
                        info!("Pipeline finished");
                        current_pipeline = None;
                        current_stream = None;
                        // fall back to prompt; UI thread will prompt on next read
                    }
                    UserEvent::PipelineOutput { output, .. } => {
                        debug!("Received pipeline output");
                        // Mirror sync's formatting path
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
                        if let Err(e) = repl_to_ui_tx.send(UiOp::Print(text)) {
                            error!("Failed to send print operation to UI thread: {}", e);
                        }
                    }
                }
            }
        }
    }

    terminate_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    info!("REPL loop ending, waiting for UI thread to finish");
    ui_handle.await??;
    info!("Async REPL completed");

    Ok(())
}
