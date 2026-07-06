use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use base64::Engine;
use ironposh_psrp::{
    ApartmentState, ApplicationArguments, ApplicationPrivateData, ConnectRunspacePool,
    CreatePipeline, Defragmenter, ErrorRecord, HostInfo, InitRunspacePool, PSThreadOptions,
    PipelineOutput, SessionCapability, fragmentation,
};
use ironposh_winrm::ws_management::{OptionSetValue, WsMan};
use rsa::RsaPrivateKey;
use tracing::{debug, info, instrument, trace, warn};

use uuid::Uuid;

use crate::{
    PwshCoreError,
    host::HostCall,
    pipeline::{Pipeline, PipelineCommand},
    powershell::PipelineHandle,
    runspace::win_rs::WinRunspace,
    runspace_pool::PsInvocationState,
};

use super::enums::RunspacePoolState;

const PROTOCOL_VERSION: &str = "2.3";
const PS_VERSION: &str = "2.0";
const SERIALIZATION_VERSION: &str = "1.1.0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredStream {
    name: String,
    command_id: Option<Uuid>,
}
impl DesiredStream {
    pub(crate) fn new(name: impl Into<String>, command_id: Option<Uuid>) -> Self {
        Self {
            name: name.into(),
            command_id,
        }
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn test_new(name: impl Into<String>, command_id: Option<Uuid>) -> Self {
        Self {
            name: name.into(),
            command_id,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command_id(&self) -> Option<&Uuid> {
        self.command_id.as_ref()
    }

    pub(crate) fn runspace_pool_streams() -> Vec<Self> {
        vec![Self {
            name: "stdout".to_string(),
            command_id: None,
        }]
    }

    pub(crate) fn pipeline_streams(command_id: Uuid) -> Vec<Self> {
        vec![Self {
            name: "stdout".to_string(),
            command_id: Some(command_id),
        }]
    }

    pub(crate) fn stdout_for_command(command_id: Uuid) -> Self {
        Self {
            name: "stdout".to_string(),
            command_id: Some(command_id),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum AcceptResponsResult {
    ReceiveResponse {
        desired_streams: Vec<DesiredStream>,
    },
    SendThenReceive {
        send_xml: String,
        desired_streams: Vec<DesiredStream>,
    },
    PipelineCreated(PipelineHandle),
    PipelineFinished(PipelineHandle),
    HostCall(HostCall),
    PipelineOutput {
        output: PipelineOutput,
        handle: PipelineHandle,
    },
    ErrorRecord {
        error_record: ErrorRecord,
        handle: PipelineHandle,
    },
    PipelineRecord {
        record: crate::psrp_record::PsrpRecord,
        handle: PipelineHandle,
    },
}

#[derive(Debug)]
pub struct RunspacePool {
    pub(super) id: uuid::Uuid,
    pub(crate) state: RunspacePoolState,
    pub(super) min_runspaces: usize,
    pub(super) max_runspaces: usize,
    pub(super) thread_options: PSThreadOptions,
    pub(super) apartment_state: ApartmentState,
    pub(super) host_info: HostInfo,
    pub(super) application_arguments: ApplicationArguments,
    pub(super) shell: WinRunspace,
    pub(super) connection: Arc<WsMan>,
    pub(super) defragmenter: Defragmenter,
    pub(super) application_private_data: Option<ApplicationPrivateData>,
    pub(super) session_capability: Option<SessionCapability>,
    pub(super) pipelines: HashMap<uuid::Uuid, Pipeline>,
    pub(super) fragmenter: fragmentation::Fragmenter,
    pub(super) desired_stream_is_pooling: bool,
    pub(super) key_exchange: Option<super::crypto::KeyExchangeState>,
    pub(super) psrp_key_exchange_pending: bool,
    pub(super) pending_host_calls: VecDeque<HostCall>,
}

impl RunspacePool {
    pub fn encrypt_secure_strings_in_value(
        &self,
        value: &mut ironposh_psrp::PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let session_key = self
            .key_exchange
            .as_ref()
            .and_then(|s| s.session_key.as_deref());
        super::crypto::encrypt_secure_strings_in_value_rec(value, session_key)
    }

    /// Build the negotiation payload shared by [`Self::open`] and
    /// [`Self::connect`]: SESSION_CAPABILITY plus the path-specific second
    /// message, fragmented into a single base64-encoded request group, with
    /// the matching `protocolversion` OptionSet.
    fn negotiation_payload(
        &mut self,
        second_message: &dyn ironposh_psrp::PsObjectWithType,
    ) -> Result<(String, OptionSetValue), crate::PwshCoreError> {
        let session_capability = SessionCapability {
            protocol_version: PROTOCOL_VERSION.to_string(),
            ps_version: PS_VERSION.to_string(),
            serialization_version: SERIALIZATION_VERSION.to_string(),
            time_zone: None,
        };

        debug!(
            session_capability = ?session_capability,
            "building negotiation payload"
        );

        let request_groups = self.fragmenter.fragment_multiple(
            &[&session_capability, second_message],
            self.id,
            None,
        )?;

        trace!(
            target: "fragmentation",
            request_groups = ?request_groups,
            group_count = request_groups.len(),
            "fragmented negotiation requests"
        );

        debug_assert!(
            request_groups.len() == 1,
            "We should have only one request group for the negotiation"
        );

        let request = request_groups
            .into_iter()
            .next()
            .ok_or(crate::PwshCoreError::UnlikelyToHappen(
                "No request group generated for negotiation",
            ))
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))?;

        let option_set = OptionSetValue::new().add_option("protocolversion", PROTOCOL_VERSION);

        Ok((request, option_set))
    }

    #[instrument(skip(self), name = "RunspacePool::open")]
    pub fn open(
        mut self,
    ) -> Result<(String, super::expect_shell_created::ExpectShellCreated), crate::PwshCoreError>
    {
        if self.state != RunspacePoolState::BeforeOpen {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in BeforeOpen state to open",
            ));
        }

        let init_runspace_pool = InitRunspacePool {
            min_runspaces: i32::try_from(self.min_runspaces).unwrap_or(i32::MAX),
            max_runspaces: i32::try_from(self.max_runspaces).unwrap_or(i32::MAX),
            thread_options: self.thread_options,
            apartment_state: self.apartment_state,
            host_info: self.host_info.clone(),
            application_arguments: self.application_arguments.clone(),
        };

        debug!(
            min_runspaces = self.min_runspaces,
            max_runspaces = self.max_runspaces,
            "starting runspace pool open"
        );
        debug!(init_runspace_pool = ?init_runspace_pool);

        let (request, option_set) = self.negotiation_payload(&init_runspace_pool)?;

        self.state = RunspacePoolState::NegotiationSent;

        let result = self
            .shell
            .open(&self.connection, Some(option_set), &request);

        Ok((
            result.into().to_xml_string()?,
            super::expect_shell_created::ExpectShellCreated {
                runspace_pool: self,
            },
        ))
    }

    /// Open this pool by attaching to an EXISTING disconnected shell
    /// (MS-WSMV 3.1.4.15 Connect / MS-PSRP 3.1.5.4).
    ///
    /// Embeds SESSION_CAPABILITY + CONNECT_RUNSPACEPOOL in the `connectXml`
    /// payload of a WSMan Connect request addressed at the shell whose id
    /// equals this pool's RPID (shell id == pool RPID in this codebase).
    #[instrument(skip(self), name = "RunspacePool::connect")]
    pub fn connect(
        mut self,
    ) -> Result<(String, super::expect_shell_connected::ExpectShellConnected), crate::PwshCoreError>
    {
        if self.state != RunspacePoolState::BeforeOpen {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in BeforeOpen state to connect",
            ));
        }

        let connect_runspace_pool = ConnectRunspacePool {
            min_runspaces: i32::try_from(self.min_runspaces).unwrap_or(i32::MAX),
            max_runspaces: i32::try_from(self.max_runspaces).unwrap_or(i32::MAX),
        };

        debug!(
            connect_runspace_pool = ?connect_runspace_pool,
            runspace_pool_id = %self.id,
            "starting runspace pool connect to existing shell"
        );

        let (request, option_set) = self.negotiation_payload(&connect_runspace_pool)?;

        self.state = RunspacePoolState::Connecting;

        let result = self
            .shell
            .connect(&self.connection, Some(option_set), &request);

        Ok((
            result.into().to_xml_string()?,
            super::expect_shell_connected::ExpectShellConnected {
                runspace_pool: self,
            },
        ))
    }

    /// Server-assigned shell id (available after the shell has been created).
    pub fn shell_id(&self) -> Option<&str> {
        self.shell.shell_id()
    }

    /// Server-supplied ApplicationPrivateData, if delivered during open/connect.
    pub fn application_private_data(&self) -> Option<&ApplicationPrivateData> {
        self.application_private_data.as_ref()
    }

    /// Abort an in-flight Disconnect after the server faulted the request.
    /// Valid only in `Disconnecting` state; reverts the pool to `Opened`.
    pub(crate) fn abort_disconnect(&mut self) {
        if self.state == RunspacePoolState::Disconnecting {
            self.state = RunspacePoolState::Opened;
            warn!(runspace_pool_id = %self.id, "disconnect aborted, runspace pool reverted to Opened");
        } else {
            warn!(
                runspace_pool_id = %self.id,
                state = ?self.state,
                "abort_disconnect called outside Disconnecting state; ignoring"
            );
        }
    }

    /// Abort an in-flight Reconnect after a transport-level failure.
    /// Valid only in `Connecting` state; reverts the pool to `Disconnected`.
    pub(crate) fn abort_reconnect(&mut self) {
        if self.state == RunspacePoolState::Connecting {
            self.state = RunspacePoolState::Disconnected;
            warn!(runspace_pool_id = %self.id, "reconnect aborted, runspace pool reverted to Disconnected");
        } else {
            warn!(
                runspace_pool_id = %self.id,
                state = ?self.state,
                "abort_reconnect called outside Connecting state; ignoring"
            );
        }
    }

    /// Compute desired streams for all currently active pipelines, plus the runspace pool stream.
    /// Used to re-issue a Receive after a timeout heartbeat.
    pub(crate) fn compute_active_desired_streams(&self) -> Vec<DesiredStream> {
        let mut streams: Vec<DesiredStream> = self
            .pipelines
            .keys()
            .map(|pipeline_id| DesiredStream::stdout_for_command(*pipeline_id))
            .collect();

        // Always include runspace pool stream if no pipeline-specific streams
        if streams.is_empty() {
            streams = DesiredStream::runspace_pool_streams();
        }

        streams
    }

    pub(crate) fn init_pipeline(
        &mut self,
        uuid: Uuid,
    ) -> Result<PipelineHandle, crate::PwshCoreError> {
        if self.pipelines.contains_key(&uuid) {
            return Err(crate::PwshCoreError::InvalidState(
                "Pipeline with this UUID already exists",
            ));
        }

        self.pipelines.insert(uuid, Pipeline::new());
        Ok(PipelineHandle { id: uuid })
    }

    #[instrument(skip_all)]
    pub fn invoke_pipeline_request(
        &mut self,
        handle: &PipelineHandle,
    ) -> Result<String, PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        // Set pipeline state to Running
        pipeline.set_state(PsInvocationState::Running);
        info!(pipeline_id = %handle.id(), "Invoking pipeline");

        // Convert business pipeline to protocol pipeline and build CreatePipeline message
        let protocol_pipeline = pipeline.to_protocol_pipeline();
        let create_pipeline = CreatePipeline::builder()
            .pipeline(protocol_pipeline)
            .host_info(self.host_info.clone())
            .apartment_state(self.apartment_state)
            .build();

        debug!(?create_pipeline);

        let fragmented =
            self.fragmenter
                .fragment(&create_pipeline, self.id, Some(handle.id()), None)?;

        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        let request = self.shell.create_pipeline_request(
            &self.connection,
            handle.id(),
            arguments,
            None,
            None,
        );

        Ok(request.into().to_xml_string()?)
    }

    /// Send a runspace pool host response to the server
    #[instrument(
        skip_all,
        fields(
            call_id = host_response.call_id,
            method = ?host_response.method
        )
    )]
    pub fn send_runspace_pool_host_response(
        &mut self,
        host_response: &ironposh_psrp::RunspacePoolHostResponse,
    ) -> Result<String, PwshCoreError> {
        let _span = tracing::trace_span!("send_runspace_pool_host_response").entered();

        // Fragment the host response message
        tracing::trace!(stage = "fragment");
        let fragmented = self.fragmenter.fragment(
            host_response,
            self.id,
            None, // No command ID for runspace pool messages
            None,
        )?;
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
        let request = self.shell.send_data_request(
            &self.connection,
            None, // No command ID for runspace pool
            &arguments,
        )?;

        tracing::trace!(stage = "wsman_send_request_built");
        let element: ironposh_xml::builder::Element<'_> = request.into();
        tracing::trace!(stage = "serialize_xml");
        let xml = element.to_xml_string().map_err(|e| {
            tracing::error!(
                error = %e,
                stage = "serialize_xml",
                "failed to serialize XML"
            );
            e
        })?;
        tracing::trace!(xml_len = xml.len(), stage = "done");
        Ok(xml)
    }

    pub(super) fn ensure_key_exchange_state(
        &mut self,
    ) -> Result<&mut super::crypto::KeyExchangeState, PwshCoreError> {
        if self.key_exchange.is_none() {
            let mut rng = rand::thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| {
                PwshCoreError::InternalError(format!("failed to generate RSA keypair: {e}"))
            })?;
            self.key_exchange = Some(super::crypto::KeyExchangeState {
                private_key,
                session_key: None,
            });
        }

        Ok(self
            .key_exchange
            .as_mut()
            .expect("key exchange state initialized"))
    }

    pub(crate) fn add_command(
        &mut self,
        powershell: &PipelineHandle,
        command: PipelineCommand,
    ) -> Result<(), PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&powershell.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        if pipeline.state() != PsInvocationState::NotStarted {
            return Err(PwshCoreError::InvalidState(
                "Cannot add to a pipeline that has already been started",
            ));
        }

        pipeline.add_command(command);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runspace_pool::RunspacePoolCreator;
    use ironposh_psrp::{HostDefaultData, Size};
    use ironposh_winrm::ws_management::SelectorSetValue;

    const SHELL_ID: &str = "2D6534D0-6B12-40E3-B773-CBA26459CFA8";

    fn test_pool(state: RunspacePoolState) -> RunspacePool {
        let size = Size {
            width: 80,
            height: 25,
        };
        let host_data = HostDefaultData::builder()
            .buffer_size(size.clone())
            .window_size(size.clone())
            .max_window_size(size.clone())
            .max_physical_window_size(size)
            .build();
        let host_info = HostInfo::builder()
            .host_default_data(host_data)
            .use_runspace_host(true)
            .build();

        let connection = Arc::new(
            WsMan::builder()
                .to("http://127.0.0.1:5985/wsman".to_string())
                .build(),
        );

        let mut pool = RunspacePoolCreator::builder()
            .host_info(host_info)
            .build()
            .into_runspace_pool(connection);

        pool.shell = WinRunspace::builder()
            .id(pool.id)
            .selector_set(SelectorSetValue::new().add_selector("ShellId", SHELL_ID))
            .build();
        pool.state = state;
        pool
    }

    fn response_envelope(action: &str, body_element: &str) -> String {
        format!(
            r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/{action}</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <a:RelatesTo>uuid:87d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        {body_element}
    </s:Body>
</s:Envelope>"#
        )
    }

    /// Envelope shape real Windows WinRM servers send for shell
    /// Disconnect/Reconnect responses: the operation is identified by the
    /// `a:Action` header and the `s:Body` is empty (no `rsp:*Response`
    /// element). Captured live from a Windows Server WinRM endpoint.
    fn empty_body_response_envelope(action: &str) -> String {
        format!(
            r#"<s:Envelope xml:lang="en-US" xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing" xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd" xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd"><s:Header><a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/{action}</a:Action><a:MessageID>uuid:D853D945-103C-47C0-933B-227915A3B45E</a:MessageID><p:OperationID s:mustUnderstand="false">uuid:9f0f3ab8-15d4-4421-a687-adac10be157f</p:OperationID><p:SequenceId>1</p:SequenceId><a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To><a:RelatesTo>uuid:e5ec8121-80b1-408f-825c-f6e2c4cccad1</a:RelatesTo></s:Header><s:Body></s:Body></s:Envelope>"#
        )
    }

    const FAULT_ENVELOPE: &str = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.dmtf.org/wbem/wsman/1/wsman/fault</a:Action>
        <a:MessageID>uuid:BB7AF8AE-D64A-422D-B36E-15A04FA17C5C</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <a:RelatesTo>uuid:bead0162-a67d-424d-9e22-4a18b6aefea8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>s:Sender</s:Value>
                <s:Subcode>
                    <s:Value>w:SchemaValidationError</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en-US">The SOAP XML in the message does not match the corresponding XML schema definition.</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#;

    #[test]
    fn fire_disconnect_requires_opened_state() {
        let mut pool = test_pool(RunspacePoolState::BeforeOpen);
        let result = pool.fire_disconnect();
        assert!(
            matches!(result, Err(PwshCoreError::InvalidState(_))),
            "fire_disconnect must fail outside Opened state, got: {result:?}"
        );
        assert_eq!(pool.state, RunspacePoolState::BeforeOpen);
    }

    #[test]
    fn fire_reconnect_requires_disconnected_state() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        let result = pool.fire_reconnect();
        assert!(
            matches!(result, Err(PwshCoreError::InvalidState(_))),
            "fire_reconnect must fail outside Disconnected state, got: {result:?}"
        );
        assert_eq!(pool.state, RunspacePoolState::Opened);
    }

    #[test]
    fn accept_disconnect_response_requires_disconnecting_state() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        let xml = response_envelope("DisconnectResponse", "<rsp:DisconnectResponse/>");
        let result = pool.accept_disconnect_response(&xml);
        assert!(
            matches!(result, Err(PwshCoreError::InvalidState(_))),
            "accept_disconnect_response must fail outside Disconnecting state, got: {result:?}"
        );
    }

    #[test]
    fn disconnect_envelope_targets_shell() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        let xml = pool.fire_disconnect().expect("fire_disconnect");

        assert!(
            xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"),
            "disconnect envelope must carry the Disconnect action, got: {xml}"
        );
        assert!(
            xml.contains(SHELL_ID),
            "disconnect envelope must carry the ShellId selector, got: {xml}"
        );
        assert!(
            xml.contains("<rsp:Disconnect"),
            "disconnect envelope must carry the rsp:Disconnect body, got: {xml}"
        );
        assert_eq!(pool.state, RunspacePoolState::Disconnecting);
    }

    #[test]
    fn disconnect_then_reconnect_roundtrip() {
        let mut pool = test_pool(RunspacePoolState::Opened);

        pool.fire_disconnect().expect("fire_disconnect");
        assert_eq!(pool.state, RunspacePoolState::Disconnecting);

        let disconnect_response =
            response_envelope("DisconnectResponse", "<rsp:DisconnectResponse/>");
        pool.accept_disconnect_response(&disconnect_response)
            .expect("accept_disconnect_response");
        assert_eq!(pool.state, RunspacePoolState::Disconnected);

        let reconnect_xml = pool.fire_reconnect().expect("fire_reconnect");
        assert!(
            reconnect_xml
                .contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Reconnect"),
            "reconnect envelope must carry the Reconnect action, got: {reconnect_xml}"
        );
        assert!(
            reconnect_xml.contains(SHELL_ID),
            "reconnect envelope must carry the ShellId selector, got: {reconnect_xml}"
        );
        assert_eq!(pool.state, RunspacePoolState::Connecting);

        let reconnect_response = response_envelope("ReconnectResponse", "<rsp:ReconnectResponse/>");
        pool.accept_reconnect_response(&reconnect_response)
            .expect("accept_reconnect_response");
        assert_eq!(pool.state, RunspacePoolState::Opened);
    }

    #[test]
    fn accept_disconnect_response_accepts_empty_body_with_action_header() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        pool.fire_disconnect().expect("fire_disconnect");

        pool.accept_disconnect_response(&empty_body_response_envelope("DisconnectResponse"))
            .expect("real-server DisconnectResponse (empty body + action header) must be accepted");
        assert_eq!(pool.state, RunspacePoolState::Disconnected);
    }

    #[test]
    fn accept_reconnect_response_accepts_empty_body_with_action_header() {
        let mut pool = test_pool(RunspacePoolState::Disconnected);
        pool.fire_reconnect().expect("fire_reconnect");

        pool.accept_reconnect_response(&empty_body_response_envelope("ReconnectResponse"))
            .expect("real-server ReconnectResponse (empty body + action header) must be accepted");
        assert_eq!(pool.state, RunspacePoolState::Opened);
    }

    #[test]
    fn accept_disconnect_response_rejects_unrelated_action_with_empty_body() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        pool.fire_disconnect().expect("fire_disconnect");

        let result =
            pool.accept_disconnect_response(&empty_body_response_envelope("ReceiveResponse"));
        assert!(
            matches!(result, Err(PwshCoreError::InvalidResponse(_))),
            "unrelated traffic must stay rejected so the tolerance logic can ignore it, got: {result:?}"
        );
        assert_eq!(pool.state, RunspacePoolState::Disconnecting);
    }

    #[test]
    fn abort_reconnect_reverts_connecting_to_disconnected() {
        let mut pool = test_pool(RunspacePoolState::Disconnected);
        pool.fire_reconnect().expect("fire_reconnect");
        assert_eq!(pool.state, RunspacePoolState::Connecting);

        pool.abort_reconnect();
        assert_eq!(pool.state, RunspacePoolState::Disconnected);
    }

    #[test]
    fn abort_reconnect_outside_connecting_is_ignored() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        pool.abort_reconnect();
        assert_eq!(
            pool.state,
            RunspacePoolState::Opened,
            "abort_reconnect must not touch the state outside Connecting"
        );
    }

    #[test]
    fn accept_disconnect_response_surfaces_fault() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        pool.fire_disconnect().expect("fire_disconnect");

        let result = pool.accept_disconnect_response(FAULT_ENVELOPE);
        assert!(
            matches!(result, Err(PwshCoreError::SoapFault { .. })),
            "a WSMan fault must surface as SoapFault, got: {result:?}"
        );
    }

    #[test]
    fn non_timeout_fault_while_pipeline_stopping_finishes_it() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        let id = uuid::Uuid::new_v4();
        let mut pipeline = Pipeline::new();
        pipeline.set_state(PsInvocationState::Stopping);
        pool.pipelines.insert(id, pipeline);

        let results = pool
            .accept_response(FAULT_ENVELOPE)
            .expect("a fault answering a Stopping pipeline must not kill the session");

        assert!(
            results
                .iter()
                .any(|r| matches!(r, AcceptResponsResult::PipelineFinished(h) if h.id == id)),
            "the stopping pipeline should be reported finished, got: {results:?}"
        );
        assert!(
            pool.pipelines.is_empty(),
            "the stopping pipeline should be removed from the pool"
        );
    }

    #[test]
    fn non_timeout_fault_without_stopping_pipeline_stays_fatal() {
        let mut pool = test_pool(RunspacePoolState::Opened);
        let result = pool.accept_response(FAULT_ENVELOPE);
        assert!(
            matches!(result, Err(PwshCoreError::SoapFault { .. })),
            "a fault unrelated to a stopping pipeline must still be fatal, got: {result:?}"
        );
    }
}
