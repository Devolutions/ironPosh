use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use base64::Engine;
use ironposh_psrp::{
    ApartmentState, ApplicationArguments, ApplicationPrivateData, CreatePipeline, Defragmenter,
    ErrorRecord, HostInfo, InitRunspacePool, PSThreadOptions, PipelineOutput, PsValue,
    RunspacePoolStateMessage, SessionCapability, fragmentation,
};
use ironposh_winrm::{
    soap::SoapEnvelope,
    ws_management::{OptionSetValue, WsMan},
};
use ironposh_xml::parser::XmlDeserialize;
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, pkcs1v15::Pkcs1v15Encrypt};
use tracing::{debug, error, info, instrument, trace, warn};

use aes::Aes256;
use cipher::block_padding::Pkcs7;
use cipher::{BlockModeEncrypt, KeyIvInit};
use uuid::Uuid;

use crate::{
    PwshCoreError,
    host::{HostCall, HostCallScope},
    pipeline::{Pipeline, PipelineCommand, PipelineSpec},
    powershell::PipelineHandle,
    runspace::win_rs::WinRunspace,
    runspace_pool::PsInvocationState,
};

use super::enums::RunspacePoolState;

const PROTOCOL_VERSION: &str = "2.3";
const PS_VERSION: &str = "2.0";
const SERIALIZATION_VERSION: &str = "1.1.0.1";

#[derive(Debug, Clone)]
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

#[expect(clippy::large_enum_variant)]
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
}

#[derive(Debug)]
pub(super) struct KeyExchangeState {
    private_key: rsa::RsaPrivateKey,
    session_key: Option<Vec<u8>>,
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
    pub(super) key_exchange: Option<KeyExchangeState>,
    pub(super) psrp_key_exchange_pending: bool,
    pub(super) pending_host_calls: VecDeque<HostCall>,
}

fn encrypt_secure_strings_in_value_rec(
    value: &mut ironposh_psrp::PsValue,
    session_key: Option<&[u8]>,
) -> Result<(), crate::PwshCoreError> {
    use ironposh_psrp::{ComplexObjectContent, Container, PsPrimitiveValue, PsValue};

    match value {
        PsValue::Primitive(PsPrimitiveValue::SecureString(bytes)) => {
            let Some(session_key) = session_key else {
                return Err(crate::PwshCoreError::InvalidResponse(
                    "SecureString encountered but PSRP session key is not established".into(),
                ));
            };
            encrypt_secure_string_bytes_in_place(bytes, session_key)?;
        }
        PsValue::Primitive(_) => {}
        PsValue::Object(obj) => {
            for prop in obj.adapted_properties.values_mut() {
                encrypt_secure_strings_in_value_rec(&mut prop.value, session_key)?;
            }
            for prop in obj.extended_properties.values_mut() {
                encrypt_secure_strings_in_value_rec(&mut prop.value, session_key)?;
            }

            match &mut obj.content {
                ComplexObjectContent::ExtendedPrimitive(p) => {
                    if let PsPrimitiveValue::SecureString(bytes) = p {
                        let Some(session_key) = session_key else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "SecureString encountered but PSRP session key is not established"
                                    .into(),
                            ));
                        };
                        encrypt_secure_string_bytes_in_place(bytes, session_key)?;
                    }
                }
                ComplexObjectContent::Container(
                    Container::Stack(items) | Container::Queue(items) | Container::List(items),
                ) => {
                    for item in items.iter_mut() {
                        encrypt_secure_strings_in_value_rec(item, session_key)?;
                    }
                }
                ComplexObjectContent::Container(Container::Dictionary(dict)) => {
                    for (_k, v) in dict.iter_mut() {
                        encrypt_secure_strings_in_value_rec(v, session_key)?;
                    }
                }
                ComplexObjectContent::Standard | ComplexObjectContent::PsEnums(_) => {}
            }
        }
    }

    Ok(())
}

fn encrypt_secure_string_bytes_in_place(
    bytes: &mut Vec<u8>,
    session_key: &[u8],
) -> Result<(), crate::PwshCoreError> {
    if session_key.len() != 32 {
        return Err(crate::PwshCoreError::InvalidResponse(
            format!(
                "PSRP SecureString encryption requires 32-byte session key; got {}",
                session_key.len()
            )
            .into(),
        ));
    }

    // PowerShell's PSRP SecureString encryption uses AES-256-CBC with a zero IV.
    // The <SS> payload is the ciphertext bytes only (base64 encoded).
    let iv = [0u8; 16];

    let encryptor = cbc::Encryptor::<Aes256>::new_from_slices(session_key, &iv).map_err(|e| {
        crate::PwshCoreError::InvalidResponse(
            format!("Failed to initialize AES encryptor: {e}").into(),
        )
    })?;

    // MS-PSRP SecureString payload is UTF-16LE plaintext encrypted with AES-256-CBC.
    let msg_len = bytes.len();
    let pad = 16 - (msg_len % 16);
    let mut buf = bytes.clone();
    buf.resize(msg_len + pad, 0);
    let ciphertext = encryptor
        .encrypt_padded::<Pkcs7>(&mut buf, msg_len)
        .map_err(|e| {
            crate::PwshCoreError::InvalidResponse(
                format!("Failed to encrypt SecureString (padding): {e}").into(),
            )
        })?;

    let out = ciphertext.to_vec();

    debug!(
        session_key_len = session_key.len(),
        plaintext_len = msg_len,
        encrypted_len = out.len(),
        "encrypted SecureString payload"
    );

    *bytes = out;
    Ok(())
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
        encrypt_secure_strings_in_value_rec(value, session_key)
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

        let session_capability = SessionCapability {
            protocol_version: PROTOCOL_VERSION.to_string(),
            ps_version: PS_VERSION.to_string(),
            serialization_version: SERIALIZATION_VERSION.to_string(),
            time_zone: None,
        };

        let init_runspace_pool = InitRunspacePool {
            min_runspaces: self.min_runspaces as i32,
            max_runspaces: self.max_runspaces as i32,
            thread_options: self.thread_options,
            apartment_state: self.apartment_state,
            host_info: self.host_info.clone(),
            application_arguments: self.application_arguments.clone(),
        };

        debug!(
            session_capability = ?session_capability,
            min_runspaces = self.min_runspaces,
            max_runspaces = self.max_runspaces,
            "starting runspace pool open"
        );
        debug!(init_runspace_pool = ?init_runspace_pool);

        let request_groups = self.fragmenter.fragment_multiple(
            &[&session_capability, &init_runspace_pool],
            self.id,
            None,
        )?;

        trace!(
            target: "fragmentation",
            request_groups = ?request_groups,
            group_count = request_groups.len(),
            "fragmented negotiation requests"
        );

        self.state = RunspacePoolState::NegotiationSent;

        debug_assert!(
            request_groups.len() == 1,
            "We should have only one request group for the opening negotiation"
        );

        let request = request_groups
            .into_iter()
            .next()
            .ok_or(crate::PwshCoreError::UnlikelyToHappen(
                "No request group generated for negotiation",
            ))
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))?;

        let option_set = OptionSetValue::new().add_option("protocolversion", PROTOCOL_VERSION);

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

    #[expect(clippy::too_many_lines)]
    #[instrument(skip(self, soap_envelope), fields(envelope_length = soap_envelope.len()))]
    pub(crate) fn accept_response(
        &mut self,
        soap_envelope: &str,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        debug!(target: "soap", "parsing SOAP envelope");

        let parsed = ironposh_xml::parser::parse(soap_envelope).map_err(|e| {
            error!(target: "xml", error = %e, xml = soap_envelope, "failed to parse XML");
            e
        })?;

        let soap_envelope = SoapEnvelope::from_node(parsed.root_element()).map_err(|e| {
            error!(target: "soap", error = %e, "failed to parse SOAP envelope");
            crate::PwshCoreError::XmlParsingError(e)
        })?;

        let mut result = Vec::new();

        if soap_envelope.body.as_ref().receive_response.is_some() {
            debug!(target: "receive", "processing receive response");

            let (streams, command_state) = WinRunspace::accept_receive_response(&soap_envelope)
                .map_err(|e| {
                    error!(target: "receive", error = %e, "failed to accept receive response");
                    e
                })?;

            let streams_ids = streams
                .iter()
                .filter_map(|stream| stream.command_id().copied())
                .collect::<Vec<_>>();

            let is_there_a_stream_has_no_command_id =
                streams.iter().any(|stream| stream.command_id().is_none());

            if is_there_a_stream_has_no_command_id {
                debug!(
                    target: "receive",
                    "stream without command_id found, should be runspace pool stream"
                );
                self.desired_stream_is_pooling = false;
            }

            debug!(
                target: "receive",
                stream_count = streams.len(),
                stream_command_ids = ?streams_ids,
                "processing streams"
            );

            let handle_results = self.handle_pwsh_responses(streams).map_err(|e| {
                error!(target: "pwsh", error = %e, "failed to handle PowerShell responses");
                e
            })?;

            let already_scheduled_receive = handle_results
                .iter()
                .any(|r| matches!(r, AcceptResponsResult::SendThenReceive { .. }));

            debug!(
                target: "pwsh",
                response_count = handle_results.len(),
                already_scheduled_receive,
                "handled PowerShell responses"
            );

            result.extend(handle_results);

            if let Some(command_state) = command_state
                && command_state.is_done()
            {
                debug!(
                    target: "pipeline",
                    pipeline_id = ?command_state.command_id,
                    "command state done received, removing pipeline"
                );
                // If command state is done, we can remove the pipeline from the pool
                let pipeline = self.pipelines.remove(&command_state.command_id);
                if pipeline.is_some() {
                    result.push(AcceptResponsResult::PipelineFinished(PipelineHandle {
                        id: command_state.command_id,
                    }));
                }
            }

            let desired_streams = if !streams_ids.is_empty() {
                // find the intersetction of streams.id and self.pipelines.keys()
                let next_desired_streams = streams_ids.into_iter().filter(|stream| {
                    self.pipelines
                        .keys()
                        .any(|pipeline_id| pipeline_id == stream)
                });

                // keep unique stream with the same id
                let mut stream_set = HashSet::new();

                for stream in next_desired_streams {
                    stream_set.insert(stream);
                }

                stream_set
                    .into_iter()
                    .map(|stream| DesiredStream::new("stdout", stream.into()))
                    .collect::<Vec<_>>()
            } else if !self.desired_stream_is_pooling {
                self.desired_stream_is_pooling = true;
                DesiredStream::runspace_pool_streams()
            } else {
                vec![]
            };

            if !already_scheduled_receive && !desired_streams.is_empty() {
                result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
            }
        }

        if soap_envelope.body.as_ref().command_response.is_some() {
            let pipeline_id = self.shell.accept_commannd_response(&soap_envelope)?;

            self.pipelines
                .get_mut(&pipeline_id)
                .ok_or_else(|| {
                    crate::PwshCoreError::InvalidResponse(
                        "Pipeline not found for command response".into(),
                    )
                })?
                .state = PsInvocationState::Running;

            result.push(AcceptResponsResult::ReceiveResponse {
                desired_streams: vec![DesiredStream::stdout_for_command(pipeline_id)],
            });

            result.push(AcceptResponsResult::PipelineCreated(PipelineHandle {
                id: pipeline_id,
            }));
        }

        if soap_envelope.body.as_ref().signal_response.is_some() {
            let pipeline_id = self.shell.accept_signal_response(&soap_envelope)?;
            match pipeline_id {
                None => {
                    // Don't know what to do with it
                }
                Some(id) => match self.pipelines.remove(&id) {
                    None => {
                        warn!(
                            target: "signal",
                            pipeline_id = ?id,
                            "received signal response for unknown pipeline"
                        );
                    }
                    Some(_) => {
                        result.push(AcceptResponsResult::PipelineFinished(PipelineHandle { id }));
                    }
                },
            }
        }

        debug!(
            target: "accept_response",
            result_count = result.len(),
            result_types = ?result.iter().map(std::mem::discriminant).collect::<Vec<_>>(),
            "accept response results"
        );

        Ok(result)
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

    /// Fire create pipeline for a specific pipeline handle (used by service API)
    #[expect(clippy::too_many_lines)]
    #[instrument(skip(self, responses))]
    fn handle_pwsh_responses(
        &mut self,
        responses: Vec<crate::runspace::win_rs::Stream>,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        let mut result = Vec::new();

        for (stream_index, stream) in responses.into_iter().enumerate() {
            debug!(
                target: "stream",
                stream_index,
                stream_name = ?stream.name(),
                pipeline_id = ?stream.command_id(),
                "processing stream"
            );

            let messages = match self.defragmenter.defragment(stream.value()).map_err(|e| {
                error!(target: "defragment", stream_index, error = %e, "failed to defragment stream");
                e
            })? {
                fragmentation::DefragmentResult::Incomplete => {
                    debug!(target: "defragment", stream_index, "stream incomplete, continuing");
                    continue;
                }
                fragmentation::DefragmentResult::Complete(power_shell_remoting_messages) => {
                    debug!(
                        target: "defragment",
                        stream_index,
                        message_count = power_shell_remoting_messages.len(),
                        "stream complete"
                    );
                    power_shell_remoting_messages
                }
            };

            for (msg_index, message) in messages.into_iter().enumerate() {
                let ps_value = message.parse_ps_message().map_err(|e| {
                    error!(
                        target: "ps_message",
                        stream_index,
                        ?message,
                        error = %e,
                        "failed to parse PS message"
                    );
                    e
                })?;

                info!(
                    target: "ps_message",
                    message_type = ?message.message_type,
                    stream_index,
                    msg_index,
                    "parsed PS message"
                );

                match message.message_type {
                    ironposh_psrp::MessageType::PublicKeyRequest => {
                        debug!(
                            target: "key_exchange",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PublicKeyRequest message"
                        );

                        // Validate the payload (best-effort).
                        if let Err(e) = ironposh_psrp::PublicKeyRequest::try_from(ps_value.clone())
                        {
                            warn!(
                                target: "key_exchange",
                                error = %e,
                                payload = ?ps_value,
                                "unexpected PublicKeyRequest payload"
                            );
                        }

                        let public_key_b64 = self.build_public_key_blob_base64()?;
                        let public_key_msg = ironposh_psrp::PublicKey {
                            public_key: public_key_b64,
                        };
                        let send_xml = self.send_runspace_pool_message(&public_key_msg)?;

                        result.push(AcceptResponsResult::SendThenReceive {
                            send_xml,
                            desired_streams: DesiredStream::runspace_pool_streams(),
                        });
                    }
                    ironposh_psrp::MessageType::EncryptedSessionKey => {
                        debug!(
                            target: "key_exchange",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling EncryptedSessionKey message"
                        );

                        let PsValue::Object(obj) = ps_value else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "Expected EncryptedSessionKey as PsValue::Object".into(),
                            ));
                        };

                        let encrypted = ironposh_psrp::EncryptedSessionKey::try_from(obj)?;
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(encrypted.encrypted_session_key)
                            .map_err(|e| {
                                crate::PwshCoreError::InvalidResponse(
                                    format!("Invalid base64 EncryptedSessionKey: {e}").into(),
                                )
                            })?;

                        if decoded.len() < 12 + 256 {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                format!(
                                    "EncryptedSessionKey blob too short: {} bytes",
                                    decoded.len()
                                )
                                .into(),
                            ));
                        }

                        let encrypted_bytes = &decoded[12..12 + 256];
                        let state = self.ensure_key_exchange_state()?;

                        let decrypted = state
                            .private_key
                            .decrypt(Pkcs1v15Encrypt, encrypted_bytes)
                            .or_else(|e| {
                                // Some stacks may provide a representation that requires reversing.
                                // Try best-effort before failing hard.
                                let mut reversed = encrypted_bytes.to_vec();
                                reversed.reverse();
                                state
                                    .private_key
                                    .decrypt(Pkcs1v15Encrypt, &reversed)
                                    .map_err(|_e2| e)
                            })
                            .map_err(|e| {
                                crate::PwshCoreError::InternalError(format!(
                                    "failed to decrypt EncryptedSessionKey: {e}"
                                ))
                            })?;

                        if decrypted.len() != 32 {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                format!(
                                    "Unexpected decrypted PSRP session key length: {} bytes",
                                    decrypted.len()
                                )
                                .into(),
                            ));
                        }

                        info!(
                            target: "key_exchange",
                            session_key_len = decrypted.len(),
                            "stored decrypted PSRP session key"
                        );
                        state.session_key = Some(decrypted);

                        self.psrp_key_exchange_pending = false;
                        while let Some(host_call) = self.pending_host_calls.pop_front() {
                            debug!(
                                target: "key_exchange",
                                host_call = ?host_call,
                                "releasing deferred host call after key exchange"
                            );
                            result.push(AcceptResponsResult::HostCall(host_call));
                        }
                    }
                    ironposh_psrp::MessageType::SessionCapability => {
                        debug!(target: "session", "handling SessionCapability message");
                        self.handle_session_capability(ps_value).map_err(|e| {
                            error!(target: "session", error = %e, "failed to handle SessionCapability");
                            e
                        })?;
                    }
                    ironposh_psrp::MessageType::ApplicationPrivateData => {
                        debug!(target: "session", "handling ApplicationPrivateData message");
                        self.handle_application_private_data(ps_value)
                            .map_err(|e| {
                                error!(target: "session", error = %e, "failed to handle ApplicationPrivateData");
                                e
                            })?;
                    }
                    ironposh_psrp::MessageType::RunspacepoolState => {
                        debug!(target: "runspace", "handling RunspacepoolState message");
                        self.handle_runspacepool_state(ps_value).map_err(|e| {
                            error!(target: "runspace", error = %e, "failed to handle RunspacepoolState");
                            e
                        })?;
                    }
                    ironposh_psrp::MessageType::ProgressRecord => {
                        debug!(
                            target: "progress",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling ProgressRecord message"
                        );
                        self.handle_progress_record(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "progress", error = %e, "failed to handle ProgressRecord");
                                e
                            })?;
                    }
                    ironposh_psrp::MessageType::InformationRecord => {
                        debug!(
                            target: "information",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling InformationRecord message"
                        );
                        self.handle_information_record(
                            ps_value,
                            stream.name(),
                            stream.command_id(),
                        )
                        .map_err(|e| {
                            error!(target: "information", error = %e, "failed to handle InformationRecord");
                            e
                        })?;
                    }
                    ironposh_psrp::MessageType::PipelineState => {
                        debug!(
                            target: "pipeline",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PipelineState message"
                        );
                        self.handle_pipeline_state(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "pipeline", error = %e, "failed to handle PipelineState");
                                e
                            })?;
                    }
                    ironposh_psrp::MessageType::PipelineHostCall => {
                        debug!(
                            target: "host_call",
                            stream_name = ?stream.name(),
                            pipeline_id = ?stream.command_id(),
                            "handling PipelineHostCall message"
                        );

                        let host_call = self
                            .handle_pipeline_host_call(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "host_call", error = %e, "failed to handle PipelineHostCall");
                                e
                            })?;
                        debug!(target: "host_call", host_call = ?host_call, "successfully created host call");

                        let needs_session_key = match &host_call {
                            HostCall::ReadLineAsSecureString { .. }
                            | HostCall::PromptForCredential1 { .. }
                            | HostCall::PromptForCredential2 { .. } => true,
                            HostCall::Prompt { transport } => {
                                let (_, _, fields) = &transport.params;
                                fields
                                    .iter()
                                    .any(|f| f.parameter_type.contains("SecureString"))
                            }
                            _ => false,
                        };

                        let has_session_key = self
                            .key_exchange
                            .as_ref()
                            .and_then(|s| s.session_key.as_ref())
                            .is_some();

                        if needs_session_key && !has_session_key {
                            info!(
                                target: "key_exchange",
                                host_call_method = host_call.method_name(),
                                "deferring host call until PSRP session key is established"
                            );
                            self.pending_host_calls.push_back(host_call);

                            if !self.psrp_key_exchange_pending {
                                self.psrp_key_exchange_pending = true;

                                info!(
                                    target: "key_exchange",
                                    "starting client-initiated PSRP key exchange"
                                );
                                let public_key_b64 = self.build_public_key_blob_base64()?;
                                let public_key_msg = ironposh_psrp::PublicKey {
                                    public_key: public_key_b64,
                                };
                                let send_xml = self.send_runspace_pool_message(&public_key_msg)?;
                                result.push(AcceptResponsResult::SendThenReceive {
                                    send_xml,
                                    desired_streams: DesiredStream::runspace_pool_streams(),
                                });
                            }
                        } else {
                            result.push(AcceptResponsResult::HostCall(host_call));
                        }
                    }
                    ironposh_psrp::MessageType::PipelineOutput => {
                        debug!(
                            target: "pipeline_output",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PipelineOutput message"
                        );

                        let output = self.handle_pipeline_output(ps_value)?;

                        debug!(target: "pipeline_output", output = ?output, "successfully handled PipelineOutput");
                        result.push(AcceptResponsResult::PipelineOutput {
                            output,
                            handle: PipelineHandle {
                                id: *stream.command_id().ok_or_else(|| {
                                    crate::PwshCoreError::InvalidResponse(
                                        "PipelineOutput message must have a command_id".into(),
                                    )
                                })?,
                            },
                        });
                    }
                    ironposh_psrp::MessageType::ErrorRecord => {
                        debug!(
                            target: "error_record",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling ErrorRecord message"
                        );

                        let PsValue::Object(complex_object) = ps_value else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "Expected ErrorRecord as PsValue::Object".into(),
                            ));
                        };

                        let error_record = ErrorRecord::try_from(complex_object).map_err(|e| {
                            error!(target: "error_record", error = %e, "failed to parse ErrorRecord");
                            e
                        })?;

                        debug!(target: "error_record", error_record = ?error_record, "successfully parsed ErrorRecord");
                        result.push(AcceptResponsResult::ErrorRecord {
                            error_record,
                            handle: PipelineHandle {
                                id: *stream.command_id().ok_or_else(|| {
                                    crate::PwshCoreError::InvalidResponse(
                                        "ErrorRecord message must have a command_id".into(),
                                    )
                                })?,
                            },
                        });
                    }
                    _ => {
                        let data_len = message.data.len();
                        let data_preview = String::from_utf8_lossy(
                            &message.data[..std::cmp::min(message.data.len(), 512)],
                        );
                        error!(
                            target: "ps_message",
                            message_type = ?message.message_type,
                            message_type_value = message.message_type.value(),
                            stream = %stream.name(),
                            command_id = ?stream.command_id(),
                            data_len,
                            data_preview = %data_preview,
                            "received message type but no handler implemented"
                        );
                        panic!(
                            "Unhandled PSRP message_type={:?} (0x{:08x}) stream={:?} command_id={:?}",
                            message.message_type,
                            message.message_type.value(),
                            stream.name(),
                            stream.command_id()
                        );
                    }
                }
            }
        }

        info!(
            target: "pwsh_responses",
            result_count = result.len(),
            "processed PowerShell responses"
        );
        Ok(result)
    }

    #[instrument(skip(self, session_capability), fields(protocol_version = tracing::field::Empty, ps_version = tracing::field::Empty))]
    fn handle_session_capability(
        &mut self,
        session_capability: PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(session_capability) = session_capability else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected SessionCapability as PsValue::Object".into(),
            ));
        };

        let session_capability = SessionCapability::try_from(session_capability)?;

        debug!(
            target: "session",
            capability = ?session_capability,
            "received SessionCapability"
        );
        self.session_capability = Some(session_capability);
        Ok(())
    }

    #[instrument(skip(self, app_data))]
    fn handle_application_private_data(
        &mut self,
        app_data: PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(app_data) = app_data else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected ApplicationPrivateData as PsValue::Object".into(),
            ));
        };

        let app_data = ApplicationPrivateData::try_from(app_data)?;
        trace!(target: "session", app_data = ?app_data, "received ApplicationPrivateData");
        self.application_private_data = Some(app_data);
        Ok(())
    }

    #[instrument(skip(self, ps_value), fields(runspace_state = tracing::field::Empty))]
    fn handle_runspacepool_state(&mut self, ps_value: PsValue) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(runspacepool_state) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected RunspacepoolState as PsValue::Object".into(),
            ));
        };

        let runspacepool_state = RunspacePoolStateMessage::try_from(runspacepool_state)?;

        // Record the state in the span
        let span = tracing::Span::current();
        span.record(
            "runspace_state",
            format!("{:?}", runspacepool_state.runspace_state),
        );

        trace!(target: "runspace", state = ?runspacepool_state, "received RunspacePoolState");

        self.state = RunspacePoolState::from(&runspacepool_state.runspace_state);

        Ok(())
    }

    #[instrument(skip(self, ps_value), fields(stream_name, command_id = ?command_id))]
    fn handle_progress_record(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(progress_record) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected ProgressRecord as PsValue::Object".into(),
            ));
        };

        let progress_record = ironposh_psrp::ProgressRecord::try_from(progress_record)?;

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        trace!(
            target: "progress",
            progress_record = ?progress_record,
            stream_name = stream_name,
            command_id = ?command_id,
            "received ProgressRecord"
        );

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;

        pipeline.add_progress_record(progress_record);

        Ok(())
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
    fn handle_information_record(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(info_record) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected InformationRecord as PsValue::Object".into(),
            ));
        };

        let info_record = ironposh_psrp::InformationRecord::try_from(info_record)?;
        trace!(
            ?info_record,
            stream_name = stream_name,
            command_id = ?command_id,
            "Received InformationRecord"
        );

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;

        pipeline.add_information_record(info_record);

        Ok(())
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
    fn handle_pipeline_state(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(pipeline_state) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected PipelineState as PsValue::Object".into(),
            ));
        };

        let pipeline_state = ironposh_psrp::PipelineStateMessage::try_from(pipeline_state)?;
        trace!(
            ?pipeline_state,
            stream_name = stream_name,
            command_id = ?command_id,
            "Received PipelineState"
        );
        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;
        // Update the pipeline state
        pipeline.state = PsInvocationState::from(pipeline_state.pipeline_state);

        Ok(())
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
        pipeline.state = PsInvocationState::Running;
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

        if pipeline.state == PsInvocationState::Stopped
            || pipeline.state == PsInvocationState::Completed
            || pipeline.state == PsInvocationState::Failed
        {
            return Err(PwshCoreError::InvalidState(
                "Cannot kill a pipeline that is already stopped, completed, or failed",
            ));
        }

        // Set pipeline state to Stopping
        pipeline.state = PsInvocationState::Stopping;
        info!(pipeline_id = %handle.id(), "Killing pipeline");

        let request = self
            .shell
            .terminal_pipeline_signal(&self.connection, handle.id())?;

        Ok(request.into().to_xml_string()?)
    }

    #[instrument(skip_all)]
    pub fn handle_pipeline_host_call(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<HostCall, crate::PwshCoreError> {
        let PsValue::Object(pipeline_host_call) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected PipelineHostCall as PsValue::Object".into(),
            ));
        };

        let pipeline_host_call = ironposh_psrp::PipelineHostCall::try_from(pipeline_host_call)?;

        debug!(
            ?pipeline_host_call,
            stream_name = stream_name,
            command_id = ?command_id,
            method_id = pipeline_host_call.method_id,
            method_name = pipeline_host_call.method_name,
            parameters = ?pipeline_host_call.parameters,
            "Received PipelineHostCall"
        );

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        let scope = HostCallScope::Pipeline {
            command_id: command_id.to_owned(),
        };

        HostCall::try_from_pipeline(scope, pipeline_host_call).map_err(|e| {
            crate::PwshCoreError::InvalidResponse(format!("Failed to parse host call: {e}").into())
        })
    }

    /// Send a pipeline host response to the server
    #[instrument(
        skip_all,
        fields(
            command_id = %command_id,
            call_id = host_response.call_id,
            method_id = host_response.method_id,
            method_name = %host_response.method_name
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

    /// Send a runspace pool host response to the server
    #[instrument(
        skip_all,
        fields(
            call_id = host_response.call_id,
            method_id = host_response.method_id,
            method_name = %host_response.method_name
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

    fn send_runspace_pool_message(
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

    fn ensure_key_exchange_state(&mut self) -> Result<&mut KeyExchangeState, PwshCoreError> {
        if self.key_exchange.is_none() {
            let mut rng = rand::thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| {
                PwshCoreError::InternalError(format!("failed to generate RSA keypair: {e}"))
            })?;
            self.key_exchange = Some(KeyExchangeState {
                private_key,
                session_key: None,
            });
        }

        Ok(self
            .key_exchange
            .as_mut()
            .expect("key exchange state initialized"))
    }

    fn build_public_key_blob_base64(&mut self) -> Result<String, PwshCoreError> {
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

    pub fn handle_pipeline_output(
        &mut self,
        ps_value: PsValue,
    ) -> Result<PipelineOutput, PwshCoreError> {
        let pipeline_output = PipelineOutput::from(ps_value);

        Ok(pipeline_output)
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

        if pipeline.state != PsInvocationState::NotStarted {
            return Err(PwshCoreError::InvalidState(
                "Cannot add to a pipeline that has already been started",
            ));
        }

        pipeline.add_command(command);
        Ok(())
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
