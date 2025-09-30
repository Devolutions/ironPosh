use crate::{
    error::WasmError, http_client::GatewayHttpViaWSClient, types::WasmWinRmConfig,
    WasmPowerShellStream,
};
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_client_core::connector::WinRmConfig;
use js_sys::Promise;
use tracing::{error, info};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

// Main PowerShell client
#[wasm_bindgen]
pub struct WasmPowerShellClient {
    client: RemoteAsyncPowershellClient,
}

#[wasm_bindgen]
impl WasmPowerShellClient {
    #[wasm_bindgen]
    pub fn connect(config: WasmWinRmConfig) -> Result<Self, WasmError> {
        info!(
            gateway_url = %config.gateway_url,
            "connecting PowerShell client"
        );

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
        let (client, task) = RemoteAsyncPowershellClient::open_task(internal_config, http_client);

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

        let stream = crate::stream::WasmPowerShellStream::new(stream);
        info!("PowerShell command stream created successfully");
        Ok(stream)
    }

    #[wasm_bindgen]
    pub fn disconnect(&self) -> Promise {
        info!("disconnecting PowerShell client");
        future_to_promise(async move {
            info!("PowerShell client disconnected");
            Ok(JsValue::NULL)
        })
    }
}
