use crate::{
    error::WasmError,
    hostcall::handle_host_calls,
    http_client::GatewayHttpViaWSClient,
    types::{SecurityWarningCallback, WasmCommandCompletion, WasmWinRmConfig},
    JsSessionEvent, WasmPowerShellStream,
};
use futures::StreamExt;
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_client_core::{connector::WinRmConfig, powershell::PipelineHandle};
use js_sys::{Array, Function, Promise};
use std::convert::TryFrom;
use tracing::{error, info, warn};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local, JsFuture};

// Main PowerShell client
#[wasm_bindgen]
pub struct WasmPowerShellClient {
    client: RemoteAsyncPowershellClient,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "(js_host_call: JsHostCall) => Promise<any> | any")]
    pub type HostCallHandler;

    #[wasm_bindgen(typescript_type = "(session_event: JsSessionEvent) => void")]
    pub type SessionEventHandler;
}

#[wasm_bindgen]
impl WasmPowerShellClient {
    /// Check the security configuration and return any warnings.
    /// Call this before connect() to inspect security status.
    #[wasm_bindgen]
    pub fn check_security(config: &WasmWinRmConfig) -> Array {
        let warnings = config.check_security();
        let arr = Array::new();
        for warning in warnings {
            let js_warning = serde_wasm_bindgen::to_value(&warning)
                .expect("Failed to serialize SecurityWarning");
            arr.push(&js_warning);
        }
        arr
    }

    /// Connect to a PowerShell session with security callback.
    ///
    /// If security warnings are detected and `on_security_warning` is provided,
    /// the callback will be invoked with the list of warnings. The callback must
    /// return a Promise<boolean>:
    /// - `true`: User accepts the risk, continue with connection
    /// - `false`: User rejects, abort connection
    ///
    /// If warnings exist but no callback is provided, connection will be rejected.
    #[wasm_bindgen]
    pub async fn connect_with_security_check(
        config: WasmWinRmConfig,
        host_call_handler: HostCallHandler,
        session_event_handler: SessionEventHandler,
        on_security_warning: Option<SecurityWarningCallback>,
    ) -> Result<Self, WasmError> {
        // Check for security warnings
        let warnings = config.check_security();

        if !warnings.is_empty() {
            // Log warnings
            for warning in &warnings {
                warn!(?warning, "security warning detected");
            }

            // If callback provided, ask user
            if let Some(callback) = on_security_warning {
                let callback_fn: Function = callback.unchecked_into();

                // Convert warnings to JS array
                let js_warnings = Array::new();
                for warning in &warnings {
                    let js_warning = serde_wasm_bindgen::to_value(warning)
                        .expect("Failed to serialize SecurityWarning");
                    js_warnings.push(&js_warning);
                }

                // Call the callback and await the promise
                let result = callback_fn
                    .call1(&JsValue::NULL, &js_warnings)
                    .map_err(|e| {
                        error!(?e, "failed to call security warning callback");
                        WasmError::Generic(format!(
                            "Failed to call security warning callback: {e:?}"
                        ))
                    })?;
                let promise = Promise::from(result);
                let should_continue = JsFuture::from(promise).await.map_err(|e| {
                    error!(?e, "security warning callback promise rejected");
                    WasmError::Generic(format!("Security warning callback rejected: {e:?}"))
                })?;

                if !should_continue.as_bool().unwrap_or(false) {
                    info!("user rejected insecure connection");
                    return Err(WasmError::Generic(
                        "Connection rejected: user declined insecure connection".into(),
                    ));
                }

                info!("user accepted insecure connection, proceeding");
            } else {
                // No callback provided, reject by default
                error!("security warnings detected but no callback provided");
                return Err(WasmError::Generic(format!(
                    "Connection rejected: security warnings detected ({warnings:?}). Provide on_security_warning callback to handle."
                )));
            }
        }

        // Proceed with connection
        Self::connect_internal(config, host_call_handler, session_event_handler)
    }

    /// Connect to a PowerShell session (legacy method, no security callback).
    /// Will reject if there are any security warnings.
    #[wasm_bindgen]
    pub fn connect(
        config: WasmWinRmConfig,
        host_call_handler: HostCallHandler,
        session_event_handler: SessionEventHandler,
    ) -> Result<Self, WasmError> {
        // Check for security warnings first
        let warnings = config.check_security();
        if !warnings.is_empty() {
            error!(
                ?warnings,
                "security warnings detected, use connect_with_security_check"
            );
            return Err(WasmError::Generic(format!(
                "Connection rejected: security warnings detected ({warnings:?}). Use connect_with_security_check() with a callback to handle warnings."
            )));
        }

        Self::connect_internal(config, host_call_handler, session_event_handler)
    }

    fn connect_internal(
        config: WasmWinRmConfig,
        host_call_handler: HostCallHandler,
        session_event_handler: SessionEventHandler,
    ) -> Result<Self, WasmError> {
        info!(
            gateway_url = %config.gateway_url,
            "connecting PowerShell client"
        );

        if !host_call_handler.is_function() || !session_event_handler.is_function() {
            error!("host_call_handler or session_event_handler is not a function");
            return Err(WasmError::InvalidArgument(
                "host_call_handler and session_event_handler must be functions".into(),
            ));
        }

        let url = Url::parse(&config.gateway_url).map_err(|e| {
            error!(
                ?e,
                gateway_url = %config.gateway_url,
                "failed to parse gateway URL"
            );
            WasmError::UrlParseError {
                source: e,
                target: config.gateway_url.clone(),
            }
        })?;

        let http_client = GatewayHttpViaWSClient::new(url, config.gateway_token.clone());
        let internal_config: WinRmConfig = config.into();
        let (client, host_io, session_event_rx, task) =
            RemoteAsyncPowershellClient::open_task(internal_config, http_client);

        // Spawn session event handler task
        spawn_local(async move {
            let mut session_event_rx = session_event_rx;
            let session_event_handler = session_event_handler.unchecked_into::<Function>();
            while let Some(event) = session_event_rx.next().await {
                let event: JsSessionEvent = event.into();
                if let Err(e) = session_event_handler.call1(&JsValue::NULL, &event.into()) {
                    error!(?e, "failed to call session event handler");
                }
            }
            info!("session event handler task exiting");
        });

        let (host_call_rx, submitter) = host_io.into_parts();

        wasm_bindgen_futures::spawn_local(handle_host_calls(
            host_call_rx,
            submitter,
            host_call_handler.unchecked_into(),
        ));

        info!("spawning background task for PowerShell client");
        // Spawn background task
        wasm_bindgen_futures::spawn_local(async move {
            #[expect(clippy::large_futures)]
            if let Err(e) = task.await {
                error!(?e, "background task failed");
                web_sys::console::error_1(&format!("Background task failed: {e}").into());
            }
        });

        info!("PowerShell client connected successfully");
        Ok(Self { client })
    }

    #[wasm_bindgen]
    pub async fn execute_command(
        &mut self,
        script: String,
    ) -> Result<WasmPowerShellStream, WasmError> {
        info!(script_length = script.len(), "executing PowerShell command");

        let stream = self.client.send_script(script).await.map_err(|e| {
            error!(?e, "failed to send PowerShell script");
            e
        })?;

        let (kill_tx, kill_rx) = futures::channel::oneshot::channel::<PipelineHandle>();
        let mut client_clone = self.client.clone();
        spawn_local(async move {
            let Ok(pipeline_handle) = kill_rx.await else {
                return;
            };

            let _ = client_clone
                .kill_pipeline(pipeline_handle)
                .await
                .inspect_err(|e| {
                    error!(?e, "failed to kill PowerShell pipeline");
                });
        });

        let stream = crate::stream::WasmPowerShellStream::new(stream, kill_tx);
        info!("PowerShell command stream created successfully");
        Ok(stream)
    }

    #[wasm_bindgen]
    pub async fn tab_complete(
        &mut self,
        input_script: String,
        cursor_column: u32,
    ) -> Result<WasmCommandCompletion, WasmError> {
        use ironposh_client_core::connector::active_session::UserEvent;

        fn escape_ps_single_quoted(input: &str) -> String {
            input.replace('\'', "''")
        }

        let escaped = escape_ps_single_quoted(&input_script);
        let script =
            format!("TabExpansion2 -inputScript '{escaped}' -cursorColumn {cursor_column}");

        info!(
            cursor_column,
            input_len = input_script.len(),
            "tab_complete: sending TabExpansion2"
        );

        let stream = self.client.send_script_raw(script).await?;
        let mut stream = stream.boxed();

        let mut output: Option<ironposh_psrp::PsValue> = None;
        let mut error_message: Option<String> = None;

        while let Some(ev) = stream.next().await {
            match ev {
                UserEvent::PipelineOutput { output: out, .. } => {
                    if output.is_none() {
                        output = Some(out.data);
                    }
                }
                UserEvent::ErrorRecord { error_record, .. } => {
                    let concise = error_record.render_concise();
                    error_message = Some(concise.clone());
                    warn!(error_message = %concise, "tab_complete: error record");
                }
                UserEvent::PipelineFinished { .. } => break,
                _ => {}
            }
        }

        let Some(ps_value) = output else {
            return Err(WasmError::Generic(
                error_message.unwrap_or_else(|| "TabExpansion2 returned no output".into()),
            ));
        };

        let completion = ironposh_psrp::CommandCompletion::try_from(&ps_value)
            .map_err(|e| WasmError::Generic(e.to_string()))?;

        info!(
            ?completion,
            cursor_column,
            replacement_index = completion.replacement_index,
            replacement_length = completion.replacement_length,
            matches = completion.completion_matches.len(),
            "tab_complete: parsed completion"
        );

        Ok(WasmCommandCompletion::from(&completion))
    }

    // pub async fn next_host_call

    #[wasm_bindgen]
    pub fn disconnect(&self) -> Promise {
        info!("disconnecting PowerShell client");
        future_to_promise(async move {
            info!("PowerShell client disconnected");
            Ok(JsValue::NULL)
        })
    }
}
