use ironposh_client_core::connector::active_session::PowershellOperations;
use ironposh_client_core::pipeline::PipelineCommand;
use ironposh_client_core::powershell::PipelineHandle;
use ironposh_client_core::connector::UserOperation;
use ironposh_psrp::PipelineOutput;
use regex::Regex;
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Handle user input for PowerShell commands (synchronous)
pub struct UserInputHandler {
    user_request_tx: mpsc::Sender<UserOperation>,
    user_event_rx: mpsc::Receiver<ironposh_client_core::connector::active_session::UserEvent>,
}

impl UserInputHandler {
    pub fn new(
        user_request_tx: mpsc::Sender<UserOperation>,
        user_event_rx: mpsc::Receiver<ironposh_client_core::connector::active_session::UserEvent>,
    ) -> Self {
        Self {
            user_request_tx,
            user_event_rx,
        }
    }

    #[instrument(skip_all, name = "user_input_handler")]
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut pipeline: Option<PipelineHandle> = None;

        info!("starting user input handler");
        self.user_request_tx
            .send(UserOperation::CreatePipeline {
                uuid: uuid::Uuid::new_v4(),
            })
            .expect("Failed to send create pipeline request");

        loop {
            // Check for user events
            match self.process_user_events(&mut pipeline) {
                PipelineOperated::Continue => continue,
                PipelineOperated::KeepGoing => {}
            }

            print!("> ");
            stdout.flush().unwrap();
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let command = line.trim().to_string();
                    if command.to_lowercase() == "exit" {
                        info!("user requested exit");
                        break;
                    }

                    if command.is_empty() {
                        continue;
                    }
                    // Ensure we have a pipeline before executing the command

                    if let Some(pipeline_handle) = pipeline {
                        // Add the script to the pipeline
                        if let Err(e) = self.user_request_tx.send(UserOperation::OperatePipeline {
                            powershell: pipeline_handle,
                            operation: PowershellOperations::AddCommand {
                                command: PipelineCommand::new_script(command),
                            },
                        }) {
                            error!(error = %e, "failed to send operation");
                            break;
                        }

                        // Invoke the pipeline
                        if let Err(e) = self.user_request_tx.send(UserOperation::InvokePipeline {
                            powershell: pipeline_handle,
                        }) {
                            error!(error = %e, "failed to send invoke");
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "failed to read input");
                    break;
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn process_user_events(&mut self, pipeline: &mut Option<PipelineHandle>) -> PipelineOperated {
        while let Ok(event) = self.user_event_rx.recv_timeout(Duration::from_millis(100)) {
            match event {
                ironposh_client_core::connector::active_session::UserEvent::PipelineCreated {
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(), "pipeline created");
                    *pipeline = Some(powershell);
                    return PipelineOperated::KeepGoing;
                }
                ironposh_client_core::connector::active_session::UserEvent::PipelineFinished {
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(), "pipeline finished");
                    if let Some(current_pipeline) = pipeline {
                        if *current_pipeline == powershell {
                            *pipeline = None;
                            self.user_request_tx
                                .send(UserOperation::CreatePipeline {
                                    uuid: Uuid::new_v4(),
                                })
                                .expect("Failed to send create pipeline request");
                        }
                    }
                }
                ironposh_client_core::connector::active_session::UserEvent::PipelineOutput {
                    output,
                    powershell,
                } => {
                    info!(pipeline_id = %powershell.id(), "pipeline output: {:?}", output);
                    if let Some(current_pipeline) = pipeline {
                        if *current_pipeline == powershell {
                            println!(
                                "{}",
                                format_pipeline_output(&output).unwrap_or_else(|e| {
                                    error!(error = %e, "failed to format pipeline output");
                                    "Error formatting output".to_string()
                                })
                            );
                        }
                    }
                }
            }
        }
        PipelineOperated::Continue
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
                            result.push_str(&format!("{high:04X}"));
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
        result.push_str(&format!("{high:04X}"));
        result.push('_');
    }

    Ok(result)
}

pub enum PipelineOperated {
    Continue,
    KeepGoing,
}
