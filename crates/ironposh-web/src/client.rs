use crate::{
    error::WasmError, hostcall::handle_host_calls, http_client::GatewayHttpViaWSClient,
    types::WasmWinRmConfig, JsSessionEvent, WasmPowerShellStream,
};
use futures::StreamExt;
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_client_core::{connector::WinRmConfig, powershell::PipelineHandle};
use js_sys::{Function, Promise};
use tracing::{error, info};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};

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
    #[wasm_bindgen]
    pub fn connect(
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

        let http_client = GatewayHttpViaWSClient::new(url, config.gateway_token.to_owned());
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
            if let Err(e) = task.await {
                error!(?e, "background task failed");
                web_sys::console::error_1(&format!("Background task failed: {}", e).into());
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
