use std::convert::TryInto;

use futures::{channel::mpsc::Receiver, StreamExt};
use ironposh_client_core::connector::active_session::UserEvent;
use tracing::{debug, error, info};
use wasm_bindgen::prelude::*;

use crate::{error::WasmError, WasmPowerShellEvent};

// PowerShell stream wrapper - this needs to be a simple struct
#[wasm_bindgen]
pub struct WasmPowerShellStream {
    // We'll use interior mutability pattern
    inner: Receiver<UserEvent>,
}

impl WasmPowerShellStream {
    pub(crate) fn new(receiver: Receiver<UserEvent>) -> Self {
        info!("creating new PowerShell stream");
        Self { inner: receiver }
    }
}

#[wasm_bindgen]
impl WasmPowerShellStream {
    #[wasm_bindgen]
    pub async fn next(&mut self) -> Result<Option<WasmPowerShellEvent>, WasmError> {
        debug!("waiting for next PowerShell event");
        let event = self.inner.next().await;
        if let Some(event) = event {
            debug!(?event, "received PowerShell event");
            let wasm_powershell_event: WasmPowerShellEvent = event.try_into().map_err(|e| {
                error!(?e, "failed to convert PowerShell event");
                e
            })?;
            debug!(
                event_type = ?wasm_powershell_event,
                "converted PowerShell event successfully"
            );
            return Ok(Some(wasm_powershell_event));
        }
        info!("PowerShell stream ended");
        Ok(None)
    }
}
