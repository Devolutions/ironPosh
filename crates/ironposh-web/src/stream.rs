use std::convert::TryInto;

use futures::{
    channel::{mpsc::Receiver, oneshot},
    StreamExt,
};
use ironposh_client_core::{connector::active_session::UserEvent, powershell::PipelineHandle};
use tracing::{debug, error, info};
use wasm_bindgen::prelude::*;

use crate::{error::WasmError, WasmPowerShellEvent};

// PowerShell stream wrapper - this needs to be a simple struct
#[wasm_bindgen]
pub struct WasmPowerShellStream {
    inner: Receiver<UserEvent>,
    pipeline_handle: Option<PipelineHandle>,
    kill_sender: Option<oneshot::Sender<PipelineHandle>>,
}

impl WasmPowerShellStream {
    pub(crate) fn new(
        receiver: Receiver<UserEvent>,
        kill_sender: oneshot::Sender<PipelineHandle>,
    ) -> Self {
        info!("creating new PowerShell stream");
        Self {
            inner: receiver,
            pipeline_handle: None,
            kill_sender: Some(kill_sender),
        }
    }
}

#[wasm_bindgen]
impl WasmPowerShellStream {
    #[wasm_bindgen]
    pub async fn next(&mut self) -> Result<Option<WasmPowerShellEvent>, WasmError> {
        let _next_span =
            tracing::span!(tracing::Level::DEBUG, "WasmPowerShellStream::next").entered();

        debug!("waiting for next PowerShell event");
        let event = self.inner.next().await;

        let result = if let Some(event) = &event {
            debug!(?event, "received PowerShell event");
            let wasm_powershell_event: WasmPowerShellEvent = event.try_into().map_err(|e| {
                error!(?e, "failed to convert PowerShell event");
                e
            })?;
            debug!(
                event_type = ?wasm_powershell_event,
                "converted PowerShell event successfully"
            );

            Ok(Some(wasm_powershell_event))
        } else {
            Ok(None)
        };

        if let Some(UserEvent::PipelineCreated { pipeline }) = event {
            self.pipeline_handle = Some(pipeline);
        }

        result
    }

    #[wasm_bindgen]
    pub fn kill(&mut self) -> Result<(), WasmError> {
        let _kill_span =
            tracing::span!(tracing::Level::INFO, "WasmPowerShellStream::kill").entered();

        let Some(pipeline_handle) = self.pipeline_handle.take() else {
            debug!("no pipeline handle available, cannot send kill signal");
            return Ok(());
        };

        let Some(kill_sender) = self.kill_sender.take() else {
            debug!("kill signal already sent, ignoring");
            return Ok(());
        };

        info!("killing PowerShell pipeline");

        kill_sender.send(pipeline_handle).map_err(|e| {
            error!(?e, "failed to send kill signal");
            WasmError::Generic("failed to send kill signal".into())
        })?;

        Ok(())
    }
}
