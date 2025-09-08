use anyhow::Context;
use byteorder::{BigEndian, ReadBytesExt};
use ironposh_client_core::connector::{
    auth_sequence::{AuthSequence, SecurityContextBuilderHolder},
    authenticator::SecContextMaybeInit,
    http::{HttpRequest, HttpResponse},
    Connector, ConnectorConfig, ConnectorStepResult,
};
use tracing::{info, instrument};

#[derive(Debug)]
pub enum KeepAlive {
    Must,
    NotNecessary,
}

pub trait HttpClient {
    fn send_request(
        &self,
        request: HttpRequest<String>,
        keep_alive: KeepAlive,
    ) -> Result<HttpResponse<String>, anyhow::Error>;
}

pub struct RemotePowershell {
    active_session: ironposh_client_core::connector::active_session::ActiveSession,
    client: Box<dyn HttpClient>,
    next_request: ironposh_client_core::connector::http::HttpRequest<String>,
}

impl RemotePowershell {
    /// Establish connection to the PowerShell remote server
    pub fn open(
        config: ConnectorConfig,
        client: impl HttpClient + 'static,
    ) -> Result<Self, anyhow::Error> {
        let mut connector = Connector::new(config);
        let mut response = None;
        let mut decryptor = None;

        let (active_session, next_request) = loop {
            let step_result = connector.step(response.take())?;

            match step_result {
                ConnectorStepResult::SendBack(http_request) => {
                    response = Some(client.send_request(http_request, KeepAlive::NotNecessary)?);
                }
                ConnectorStepResult::SendBackError(e) => {
                    anyhow::bail!("Connection failed: {}", e);
                }
                ConnectorStepResult::Connected {
                    active_session,
                    next_receive_request,
                } => {
                    break (*active_session, next_receive_request);
                }
                ConnectorStepResult::Auth { mut sequence } => {
                    let mut auth_response = None;
                    // Authentication sequence handling - mimic auth_sequence.rs pattern
                    let final_token = loop {
                        let sec_ctx_init = {
                            let mut holder = SecurityContextBuilderHolder::new();
                            let result = sequence
                                .try_init_sec_context(auth_response.as_ref(), &mut holder)?;

                            match result {
                                SecContextMaybeInit::Initialized(sec_context_init) => {
                                    sec_context_init
                                }
                                SecContextMaybeInit::RunGenerator {
                                    mut packet,
                                    mut generator_holder,
                                } => loop {
                                    let kdc_response = send_packet(packet)?;
                                    match AuthSequence::resume(generator_holder, kdc_response)? {
                                        SecContextMaybeInit::Initialized(sec_context_init) => {
                                            break sec_context_init;
                                        }
                                        SecContextMaybeInit::RunGenerator {
                                            packet: packet2,
                                            generator_holder: generator2,
                                        } => {
                                            packet = packet2;
                                            generator_holder = generator2;
                                        }
                                    }
                                },
                            }
                        };

                        let action = sequence.process_initialized_sec_context(sec_ctx_init)?;

                        match action {
                            ironposh_client_core::connector::auth_sequence::SecCtxInited::Continue(http_request) => {
                                auth_response = Some(client.send_request(http_request,KeepAlive::Must)?);
                            }
                            ironposh_client_core::connector::auth_sequence::SecCtxInited::Done(token) => {
                                break token;
                            },
                        }
                    };

                    let (decryptor_inner, http_builder) = sequence.destruct_for_next_step();

                    decryptor = Some(decryptor_inner);
                    let request = connector.authenticate(None, http_builder)?;
                    response = Some(client.send_request(request, KeepAlive::Must)?);
                }
            }
        };

        Ok(Self {
            active_session,
            client: Box::new(client),
            next_request,
        })
    }

    /// Extract the components for use in the main event loop
    pub fn into_components(
        self,
    ) -> (
        ironposh_client_core::connector::active_session::ActiveSession,
        ironposh_client_core::connector::http::HttpRequest<String>,
    ) {
        (self.active_session, self.next_request)
    }
}

pub struct KerberosPacketSender {
    stream: Option<std::net::TcpStream>,
}

impl KerberosPacketSender {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

#[instrument]
fn send_packet(
    packet: ironposh_client_core::connector::NetworkRequest,
) -> Result<Vec<u8>, anyhow::Error> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, UdpSocket};
    use std::time::Duration;

    info!(protocol = ?packet.protocol, url = %packet.url, len = packet.data.len(), "Sending packet to KDC");

    match packet.protocol {
        ironposh_client_core::connector::NetworkProtocol::Tcp => {
            // TCP implementation for Kerberos KDC communication
            let host = packet
                .url
                .host_str()
                .ok_or_else(|| anyhow::anyhow!("Missing host in URL"))?;
            let port = packet
                .url
                .port()
                .ok_or_else(|| anyhow::anyhow!("Missing port in URL"))?;

            // Establish TCP connection to the KDC
            let mut stream = TcpStream::connect((host, port))
                .context("trying to establish TCP connection to KDC")?;

            stream
                .write_all(&packet.data)
                .map_err(|e| anyhow::anyhow!("Failed to write packet data: {}", e))?;

            stream
                .flush()
                .map_err(|e| anyhow::anyhow!("Failed to flush stream: {}", e))?;

            // Read the response length (4 bytes, big-endian)
            let response_len = stream
                .read_u32::<BigEndian>()
                .map_err(|e| anyhow::anyhow!("Failed to read response length: {}", e))?;

            // Read the response data
            let mut response_data = vec![0u8; response_len as usize + 4];
            response_data[..4].copy_from_slice(&response_len.to_be_bytes()); // include length prefix in re

            stream
                .read_exact(&mut response_data[4..])
                .map_err(|e| anyhow::anyhow!("Failed to read response data: {}", e))?;

            Ok(response_data)
        }

        ironposh_client_core::connector::NetworkProtocol::Udp => {
            todo!()
        }

        ironposh_client_core::connector::NetworkProtocol::Http => {
            todo!()
        }

        ironposh_client_core::connector::NetworkProtocol::Https => {
            todo!()
        }
    }
}
