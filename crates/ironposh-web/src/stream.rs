use std::convert::TryInto;

use futures::{channel::mpsc::Receiver, StreamExt};
use ironposh_client_core::connector::active_session::UserEvent;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::{error::WasmError, WasmPowerShellEvent};

// PowerShell stream wrapper - this needs to be a simple struct
#[wasm_bindgen]
pub struct WasmPowerShellStream {
    // We'll use interior mutability pattern
    inner: Receiver<UserEvent>,
}

impl WasmPowerShellStream {
    pub(crate) fn new(receiver: Receiver<UserEvent>) -> Self {
        Self { inner: receiver }
    }
}

#[wasm_bindgen]
impl WasmPowerShellStream {
    #[wasm_bindgen]
    pub async fn next(&mut self) -> Result<Option<WasmPowerShellEvent>, WasmError> {
        let event = self.inner.next().await;
        if let Some(event) = event {
            let wasm_powershell_event: WasmPowerShellEvent = event.try_into()?;
            return Ok(Some(wasm_powershell_event));
        }
        Ok(None)
    }
}
