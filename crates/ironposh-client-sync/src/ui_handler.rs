use anyhow::Context;
use ironposh_client_core::connector::active_session::{self};
use ironposh_client_core::connector::UserOperation;
use ironposh_client_core::pipeline::{PipelineCommand, PipelineSpec};
use ironposh_terminal::{ReadOutcome, Terminal};
use std::io::Write;
use std::sync::mpsc;
use tracing::info;

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

        'ui: loop {
            match io.read_line("> ")? {
                ReadOutcome::Line(cmd) => {
                    let command = cmd.trim();
                    if command.eq_ignore_ascii_case("exit") {
                        break Ok(());
                    }
                    if command.is_empty() {
                        continue;
                    }

                    // Create and invoke pipeline in one operation
                    let spec = PipelineSpec {
                        commands: vec![
                            PipelineCommand::new_script(command.to_string()),
                            PipelineCommand::new_output_stream(),
                        ],
                    };

                    info!(command = %command, "invoking pipeline with spec");
                    self.user_request_tx
                        .send(UserOperation::InvokeWithSpec {
                            uuid: uuid::Uuid::new_v4(),
                            spec,
                        })
                        .context("Failed to send invoke with spec operation")?;
                }
                ReadOutcome::Interrupt => {
                    continue; // reprompt (like shells)
                }
                ReadOutcome::Eof => break Ok(()),
            }

            'receive: while let Ok(event) = self.unified_rx.recv() {
                let read_outcome = io.try_read_line()?;
                if let Some(ReadOutcome::Interrupt) = read_outcome {
                    // User pressed Ctrl+C while waiting for events
                    continue 'receive;
                }

                match event {
                    UIInputEvent::UiOp(op) => {
                        match op {
                            UiOp::Apply(ops) => {
                                for o in ops {
                                    io.apply_op(o);
                                }
                            }
                            UiOp::Print(s) => {
                                use std::io::Write;
                                writeln!(io, "{s}")?;
                            }
                        }
                        io.render()?;
                        continue 'receive;
                    }
                    UIInputEvent::UserEvent(user_event) => {
                        match user_event {
                            active_session::UserEvent::PipelineCreated { pipeline: _ } => {
                                // Internal event, no action needed in the new simplified API
                                continue 'receive;
                            }
                            active_session::UserEvent::PipelineFinished { pipeline: _ } => {
                                // Pipeline finished, ready for next command
                                continue 'ui;
                            }
                            active_session::UserEvent::PipelineOutput {
                                output,
                                pipeline: _,
                            } => {
                                match output.format_as_displyable_string() {
                                    Ok(o) => {
                                        let _ = writeln!(io, "{o}");
                                    }
                                    Err(e) => {
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
