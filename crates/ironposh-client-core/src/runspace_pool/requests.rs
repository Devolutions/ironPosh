//! Outbound WSMan request builders for [`RunspacePool`].
//!
//! This file is a continuation of the `impl RunspacePool` block that lives in
//! [`super::pool`]. It groups the methods that build outbound WSMan requests
//! (Receive / Disconnect / Reconnect / Signal / Send) and the PSRP message
//! serialization that feeds them, so that all `&self.connection` (WsMan) usage
//! is collected in one place as groundwork for a future transport seam.
//!
//! This is a pure file-organization split: the methods here are behavior- and
//! signature-identical to their previous definitions in `pool.rs`.

use base64::Engine;
use rsa::traits::PublicKeyParts;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    PwshCoreError, pipeline::PipelineSpec, powershell::PipelineHandle,
    runspace_pool::PsInvocationState,
};

use super::enums::RunspacePoolState;
use super::pool::{DesiredStream, RunspacePool};

impl RunspacePool {
    // We should accept the pipeline id here, but for now let's ignore it
    pub(crate) fn fire_receive(
        &self,
        desired_streams: Vec<DesiredStream>,
    ) -> Result<String, crate::PwshCoreError> {
        debug_assert!(!desired_streams.is_empty(), "At least one desired stream");
        Ok(self
            .shell
            .fire_receive(&self.connection, desired_streams)
            .into()
            .to_xml_string()?)
    }

    /// Build a Disconnect request for this pool's shell (MS-WSMV 3.1.4.13).
    /// Valid only in `Opened` state; transitions the pool to `Disconnecting`.
    #[instrument(skip(self))]
    pub fn fire_disconnect(&mut self) -> Result<String, crate::PwshCoreError> {
        if self.state != RunspacePoolState::Opened {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Opened state to disconnect",
            ));
        }

        let xml = self
            .shell
            .fire_disconnect(&self.connection)
            .into()
            .to_xml_string()?;

        self.state = RunspacePoolState::Disconnecting;
        info!(runspace_pool_id = %self.id, "runspace pool disconnect requested");
        Ok(xml)
    }

    /// Build a Reconnect request for this pool's shell (MS-WSMV 3.1.4.14).
    /// Valid only in `Disconnected` state; transitions the pool to `Connecting`.
    #[instrument(skip(self))]
    pub fn fire_reconnect(&mut self) -> Result<String, crate::PwshCoreError> {
        if self.state != RunspacePoolState::Disconnected {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Disconnected state to reconnect",
            ));
        }

        let xml = self
            .shell
            .fire_reconnect(&self.connection)
            .into()
            .to_xml_string()?;

        self.state = RunspacePoolState::Connecting;
        info!(runspace_pool_id = %self.id, "runspace pool reconnect requested");
        Ok(xml)
    }

    pub fn kill_pipeline(&mut self, handle: &PipelineHandle) -> Result<String, PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState(
                "Pipeline handle not found, pipeline_id",
            ))
            .inspect_err(|_| {
                error!(pipeline_id = ?&handle.id(), "Pipeline handle not found ");
            })?;

        if pipeline.is_terminal() {
            return Err(PwshCoreError::InvalidState(
                "Cannot kill a pipeline that is already stopped, completed, or failed",
            ));
        }

        // Set pipeline state to Stopping
        pipeline.set_state(PsInvocationState::Stopping);
        info!(pipeline_id = %handle.id(), "Killing pipeline");

        let request = self
            .shell
            .terminal_pipeline_signal(&self.connection, handle.id())?;

        Ok(request.into().to_xml_string()?)
    }

    /// Send a pipeline host response to the server
    #[instrument(
        skip_all,
        fields(
            command_id = %command_id,
            call_id = host_response.call_id,
            method = ?host_response.method
        )
    )]
    pub fn send_pipeline_host_response(
        &mut self,
        command_id: uuid::Uuid,
        host_response: &ironposh_psrp::PipelineHostResponse,
    ) -> Result<String, PwshCoreError> {
        let _span = tracing::trace_span!("send_pipeline_host_response").entered();

        // Fragment the host response message
        tracing::trace!(stage = "fragment");
        let fragmented =
            self.fragmenter
                .fragment(host_response, self.id, Some(command_id), None)?;
        tracing::trace!(fragment_count = fragmented.len(), stage = "fragmented");

        // Encode fragments as base64
        tracing::trace!(stage = "base64_encode");
        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();
        tracing::trace!(
            argument_count = arguments.len(),
            first_len = arguments.first().map(String::len),
            stage = "base64_encoded"
        );

        // Create WS-Man Send request (send data to stdin)
        tracing::trace!(stage = "wsman_send_request");
        let request =
            self.shell
                .send_data_request(&self.connection, Some(command_id), &arguments)?;
        tracing::trace!(stage = "wsman_send_request_built");

        let element: ironposh_xml::builder::Element<'_> = request.into();
        tracing::trace!(stage = "serialize_xml");
        let xml = element.to_xml_string().map_err(|e| {
            tracing::error!(error = %e, stage = "serialize_xml", "failed to serialize XML");
            e
        })?;
        tracing::trace!(xml_len = xml.len(), stage = "done");
        Ok(xml)
    }

    pub(super) fn send_runspace_pool_message(
        &mut self,
        message: &dyn ironposh_psrp::PsObjectWithType,
    ) -> Result<String, PwshCoreError> {
        let _span = tracing::trace_span!(
            "send_runspace_pool_message",
            message_type = ?message.message_type()
        )
        .entered();

        tracing::trace!(stage = "fragment");
        let fragmented = self.fragmenter.fragment(message, self.id, None, None)?;
        tracing::trace!(fragment_count = fragmented.len(), stage = "fragmented");

        tracing::trace!(stage = "base64_encode");
        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        tracing::trace!(stage = "wsman_send_request");
        let request = self
            .shell
            .send_data_request(&self.connection, None, &arguments)?;

        let element: ironposh_xml::builder::Element<'_> = request.into();
        let xml = element.to_xml_string()?;
        Ok(xml)
    }

    pub(super) fn build_public_key_blob_base64(&mut self) -> Result<String, PwshCoreError> {
        const MAGIC: [u8; 4] = [0x06, 0x02, 0x00, 0x00];
        const KEYTYPE: [u8; 4] = [0x00, 0xA4, 0x00, 0x00];
        const RSA1: [u8; 4] = [0x52, 0x53, 0x41, 0x31];
        const BITLEN_2048: [u8; 4] = [0x00, 0x08, 0x00, 0x00];

        let state = self.ensure_key_exchange_state()?;
        let public_key = state.private_key.to_public_key();

        let exponent_be_raw = public_key.e().to_bytes_be();
        if exponent_be_raw.is_empty() || exponent_be_raw.len() > 4 {
            return Err(PwshCoreError::InternalError(format!(
                "unexpected RSA exponent length: {} bytes",
                exponent_be_raw.len()
            )));
        }
        let mut exponent_be_padded = [0u8; 4];
        exponent_be_padded[4 - exponent_be_raw.len()..].copy_from_slice(&exponent_be_raw);
        let exponent_u32 = u32::from_be_bytes(exponent_be_padded);
        let exponent_le_u32_bytes = exponent_u32.to_le_bytes();

        let mut modulus_be = public_key.n().to_bytes_be();
        if modulus_be.len() > 256 {
            return Err(PwshCoreError::InternalError(format!(
                "RSA modulus too large: {} bytes",
                modulus_be.len()
            )));
        }
        if modulus_be.len() < 256 {
            let mut padded = vec![0u8; 256 - modulus_be.len()];
            padded.extend_from_slice(&modulus_be);
            modulus_be = padded;
        }
        let modulus_le_bytes = modulus_be.into_iter().rev().collect::<Vec<u8>>();

        let mut blob = Vec::with_capacity(4 + 4 + 4 + 4 + 4 + 256);
        blob.extend_from_slice(&MAGIC);
        blob.extend_from_slice(&KEYTYPE);
        blob.extend_from_slice(&RSA1);
        blob.extend_from_slice(&BITLEN_2048);
        blob.extend_from_slice(&exponent_le_u32_bytes);
        blob.extend_from_slice(&modulus_le_bytes);

        Ok(base64::engine::general_purpose::STANDARD.encode(blob))
    }

    /// Create, populate, and invoke a pipeline in one operation
    pub(crate) fn invoke_spec(
        &mut self,
        uuid: Uuid,
        spec: PipelineSpec,
    ) -> Result<String, PwshCoreError> {
        // 1) Create the pipeline
        let handle = self.init_pipeline(uuid)?;

        // 2) Add all commands from the spec
        for cmd in spec.commands {
            self.add_command(&handle, cmd)?;
        }

        // 3) Invoke the pipeline using existing logic
        self.invoke_pipeline_request(&handle)
    }
}
