use anyhow::Context;
use ironposh_client_core::connector::active_session::{self};
use ironposh_client_core::connector::UserOperation;
use ironposh_client_core::pipeline::{PipelineCommand, PipelineSpec};
use ironposh_client_core::powershell::PipelineHandle;
use ironposh_terminal::{ReadOutcome, Terminal};
use std::io::Write;
use std::sync::mpsc;
use tracing::{debug, info};

use crate::types::{UIInputEvent, UiOp};

/// Handle user input for PowerShell commands (synchronous)
pub struct UIHanlder {
    user_request_tx: mpsc::Sender<UserOperation>,
    unified_rx: mpsc::Receiver<UIInputEvent>,
}

impl UIHanlder {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        unified_rx: mpsc::Receiver<UIInputEvent>,
    ) -> Self {
        Self {
            user_request_tx,
            unified_rx,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> anyhow::Result<()> {
        let mut io = terminal.stdio(); // stdio-like wrapper
        let mut current_pipeline: Option<PipelineHandle> = None;

        let _ui_loop_span = tracing::span!(tracing::Level::INFO, "ui_loop").entered();
        info!("Starting UI loop");
        'ui: loop {
            debug!("Waiting for user input");
            match io.read_line("> ")? {
                ReadOutcome::Line(cmd) => {
                    let command = cmd.trim();
                    debug!(command = %command, "Received command input");
                    if command.eq_ignore_ascii_case("exit") {
                        info!("Exit command received, terminating UI loop");
                        break Ok(());
                    }
                    if command.is_empty() {
                        debug!("Empty command received, continuing");
                        continue;
                    }

                    // Create and invoke pipeline in one operation
                    let spec = PipelineSpec {
                        commands: vec![
                            PipelineCommand::new_script(command.to_string()),
                            PipelineCommand::new_output_stream(),
                        ],
                    };

                    let uuid = uuid::Uuid::new_v4();
                    info!(command = %command, pipeline_uuid = %uuid, "invoking pipeline with spec");
                    self.user_request_tx
                        .send(UserOperation::InvokeWithSpec { uuid, spec })
                        .context("Failed to send invoke with spec operation")?;
                    debug!("Pipeline request sent successfully");
                }
                ReadOutcome::Interrupt => {
                    debug!("Interrupt received, reprompting");
                    continue; // reprompt (like shells)
                }
                ReadOutcome::Eof => {
                    info!("EOF received, terminating UI loop");
                    break Ok(());
                }
            }

            debug!("Entering event receive loop");
            'receive: while let Ok(event) = self.unified_rx.recv() {
                debug!("Received UI event");
                let read_outcome = io.try_read_line()?;
                if let Some(ReadOutcome::Interrupt) = read_outcome {
                    debug!("Interrupt detected during event processing");
                    let Some(pipeline) = current_pipeline.take() else {
                        debug!("No active pipeline to interrupt, continuing");
                        continue 'ui;
                    };

                    info!(pipeline_id = %pipeline.id(), "sending interrupt to pipeline");

                    self.user_request_tx
                        .send(UserOperation::KillPipeline { pipeline })
                        .context("Failed to send interrupt operation")?;

                    debug!("Interrupt operation sent successfully");
                    continue 'receive;
                }
                match event {
                    UIInputEvent::UiOp(op) => {
                        debug!("Processing UI operation");
                        match op {
                            UiOp::Apply(ops) => {
                                debug!(op_count = ops.len(), "Applying terminal operations");
                                for o in ops {
                                    io.apply_op(o);
                                }
                            }
                            UiOp::Print(s) => {
                                debug!(text = %s, "Printing text to terminal");
                                use std::io::Write;
                                writeln!(io, "{s}")?;
                            }
                        }
                        io.render()?;
                        debug!("UI operation processed and rendered");
                        continue 'receive;
                    }
                    UIInputEvent::UserEvent(user_event) => {
                        debug!("Processing user event");
                        match user_event {
                            active_session::UserEvent::PipelineCreated { pipeline } => {
                                info!(pipeline_id = %pipeline.id(), "Pipeline created, setting as current");
                                current_pipeline = Some(pipeline);
                                continue 'receive;
                            }
                            active_session::UserEvent::PipelineFinished { pipeline: _ } => {
                                info!("Pipeline finished, clearing current pipeline");
                                current_pipeline = None;
                                debug!("Returning to UI input loop");
                                continue 'ui;
                            }
                            active_session::UserEvent::PipelineOutput {
                                output,
                                pipeline: _,
                            } => {
                                debug!("Received pipeline output");
                                match output.format_as_displyable_string() {
                                    Ok(o) => {
                                        debug!(
                                            output_length = o.len(),
                                            "Successfully formatted pipeline output"
                                        );
                                        let _ = writeln!(io, "{o}");
                                    }
                                    Err(e) => {
                                        debug!(error = %e, "Failed to format pipeline output");
                                        let _ = writeln!(io, "Error formatting output: {e}");
                                    }
                                };
                                let _ = io.render(); // best-effort
                                continue 'receive;
                            }
                        }
                    }
                }
            }
        }
    }
}
