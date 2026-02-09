use js_sys::Function;
use tracing::info;
use wasm_bindgen::prelude::*;

use crate::{
    error::WasmError,
    types::{
        SecurityWarningCallback, WasmCommandCompletion, WasmInformationMessageData,
        WasmPowerShellEvent, WasmPsrpRecord, WasmWinRmConfig,
    },
    WasmPowerShellClient,
};

fn noop_js_function() -> Function {
    // A no-op handler for non-interactive runner usage.
    Function::new_no_args("return undefined;")
}

fn format_record(record: &WasmPsrpRecord) -> String {
    match record {
        WasmPsrpRecord::Debug { message, .. } => format!("[debug] {message}"),
        WasmPsrpRecord::Verbose { message, .. } => format!("[verbose] {message}"),
        WasmPsrpRecord::Warning { message, .. } => format!("[warning] {message}"),
        WasmPsrpRecord::Information { message_data, .. } => {
            let text = match message_data {
                WasmInformationMessageData::String { value } => value.clone(),
                WasmInformationMessageData::HostInformationMessage { value } => {
                    value.message.clone()
                }
                WasmInformationMessageData::Object { value } => format!("{value:?}"),
            };
            format!("[information] {text}")
        }
        WasmPsrpRecord::Progress {
            activity,
            status_description,
            percent_complete,
            ..
        } => {
            let status = status_description.clone().unwrap_or_default();
            format!("[progress] {activity}: {status} ({percent_complete}%)")
        }
        WasmPsrpRecord::Unsupported { data_preview, .. } => format!("[unsupported] {data_preview}"),
    }
}

#[wasm_bindgen]
pub struct WasmPowerShellRunner {
    client: Option<WasmPowerShellClient>,
}

impl Default for WasmPowerShellRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPowerShellRunner {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { client: None }
    }

    #[wasm_bindgen]
    pub async fn connect(
        &mut self,
        config: WasmWinRmConfig,
        on_security_warning: Option<SecurityWarningCallback>,
    ) -> Result<(), WasmError> {
        if self.client.is_some() {
            return Err(WasmError::Generic(
                "Runner is already connected".to_string(),
            ));
        }

        info!(
            has_security_callback = ?on_security_warning.is_some(),
            gateway_url = %config.gateway_url,
            "runner connect requested"
        );

        let host_call_handler = noop_js_function();
        let session_event_handler = noop_js_function();

        let client = if on_security_warning.is_some() {
            WasmPowerShellClient::connect_with_security_check(
                config,
                host_call_handler.unchecked_into(),
                session_event_handler.unchecked_into(),
                on_security_warning,
            )
            .await?
        } else {
            WasmPowerShellClient::connect(
                config,
                host_call_handler.unchecked_into(),
                session_event_handler.unchecked_into(),
            )?
        };

        self.client = Some(client);
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn run_command(&mut self, command: String) -> Result<Vec<String>, WasmError> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| WasmError::Generic("Runner is not connected".to_string()))?;

        let mut stream = client.execute_command(command).await?;
        let mut lines: Vec<String> = Vec::new();

        loop {
            let Some(event) = stream.next().await? else {
                break;
            };

            match event {
                WasmPowerShellEvent::PipelineOutput { data, .. } => {
                    lines.push(data);
                }
                WasmPowerShellEvent::PipelineError { error, .. } => {
                    return Err(WasmError::Generic(error.normal_formated_message));
                }
                WasmPowerShellEvent::PipelineRecord { record, .. } => {
                    lines.push(format_record(&record));
                }
                WasmPowerShellEvent::PipelineFinished { .. } => {
                    break;
                }
                WasmPowerShellEvent::PipelineCreated { .. } => {}
            }
        }

        Ok(lines)
    }

    #[wasm_bindgen]
    pub async fn tab_complete(
        &mut self,
        input_script: String,
        cursor_column: u32,
    ) -> Result<WasmCommandCompletion, WasmError> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| WasmError::Generic("Runner is not connected".to_string()))?;

        client.tab_complete(input_script, cursor_column).await
    }

    #[wasm_bindgen]
    pub fn close(&mut self) -> Result<(), WasmError> {
        self.client = None;
        Ok(())
    }
}
