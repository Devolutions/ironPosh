use crate::{
    error::WasmError, http_client::GatewayHttpViaWSClient, types::WasmWinRmConfig,
    WasmPowerShellStream,
};
use ironposh_async::RemoteAsyncPowershellClient;
use ironposh_client_core::connector::WinRmConfig;
use js_sys::Promise;
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
        let url = Url::parse(&config.gateway_url).map_err(|e| WasmError::UrlParseError {
            source: e,
            target: config.gateway_url.clone(),
        })?;

        let http_client = GatewayHttpViaWSClient::new(url, config.gateway_token.to_owned());
        let internal_config: WinRmConfig = config.into();
        let (client, task) = RemoteAsyncPowershellClient::open_task(internal_config, http_client);
        // Spawn background task
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = task.await {
                web_sys::console::error_1(&format!("Background task failed: {}", e).into());
            }
        });

        Ok(Self { client })
    }

    #[wasm_bindgen]
    pub async fn execute_command(
        &mut self,
        script: String,
    ) -> Result<WasmPowerShellStream, WasmError> {
        let stream = self.client.send_script(script).await?;

        let stream = crate::stream::WasmPowerShellStream::new(stream);
        Ok(stream)
    }

    #[wasm_bindgen]
    pub fn disconnect(&self) -> Promise {
        future_to_promise(async move { Ok(JsValue::NULL) })
    }
}
