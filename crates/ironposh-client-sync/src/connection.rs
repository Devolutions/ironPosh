use ironposh_client_core::connector::{
    auth_sequence::{AuthSequence, SecurityContextBuilderHolder},
    authenticator::SecContextMaybeInit,
    http::{HttpRequest, HttpResponse},
    Connector, ConnectorConfig, ConnectorStepResult,
};

pub trait HttpClient {
    fn send_request(
        &self,
        request: HttpRequest<String>,
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

        let (active_session, next_request) = loop {
            let step_result = connector.step(response.take())?;

            match step_result {
                ConnectorStepResult::SendBack(http_request) => {
                    response = Some(client.send_request(http_request)?);
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
                    // Authentication sequence handling - mimic auth_sequence.rs pattern
                    let _final_token = loop {
                        let sec_ctx_init = {
                            let mut holder = SecurityContextBuilderHolder::new();
                            let result =
                                sequence.try_init_sec_context(response.as_ref(), &mut holder)?;

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
                                response = Some(client.send_request(http_request)?);
                            }
                            ironposh_client_core::connector::auth_sequence::SecCtxInited::Done(token) => {
                                break token;
                            },
                        }
                    };

                    // After authentication is complete, continue with the connector step
                    response = None; // Reset response for next iteration
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

fn send_packet(
    packet: ironposh_client_core::connector::NetworkRequest,
) -> Result<Vec<u8>, anyhow::Error> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, UdpSocket};
    use std::time::Duration;

    match packet.protocol {
        ironposh_client_core::connector::NetworkProtocol::Tcp => {
            // TCP implementation for Kerberos KDC communication
            let host = packet.url.host_str()
                .ok_or_else(|| anyhow::anyhow!("Missing host in URL"))?;
            let port = packet.url.port()
                .ok_or_else(|| anyhow::anyhow!("Missing port in URL"))?;

            // Establish TCP connection to the KDC
            let mut stream = TcpStream::connect((host, port))
                .map_err(|e| anyhow::anyhow!("Failed to connect to KDC at {}:{}: {}", host, port, e))?;

            // Set timeouts for the connection
            stream.set_read_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| anyhow::anyhow!("Failed to set read timeout: {}", e))?;
            stream.set_write_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| anyhow::anyhow!("Failed to set write timeout: {}", e))?;

            // For Kerberos TCP transport (RFC 4120), the packet is prefixed with a 4-byte length field
            // in network byte order (big-endian)
            let packet_len = packet.data.len() as u32;
            let length_prefix = packet_len.to_be_bytes();

            // Send the length prefix followed by the packet data
            stream.write_all(&length_prefix)
                .map_err(|e| anyhow::anyhow!("Failed to write packet length: {}", e))?;
            stream.write_all(&packet.data)
                .map_err(|e| anyhow::anyhow!("Failed to write packet data: {}", e))?;
            stream.flush()
                .map_err(|e| anyhow::anyhow!("Failed to flush stream: {}", e))?;

            // Read the response length (4 bytes, big-endian)
            let mut length_buf = [0u8; 4];
            stream.read_exact(&mut length_buf)
                .map_err(|e| anyhow::anyhow!("Failed to read response length: {}", e))?;

            let response_len = u32::from_be_bytes(length_buf) as usize;

            // Validate response length to prevent excessive memory allocation
            if response_len > 65536 {  // 64KB max response size
                return Err(anyhow::anyhow!("Response too large: {} bytes", response_len));
            }

            // Read the response data
            let mut response_data = vec![0u8; response_len];
            stream.read_exact(&mut response_data)
                .map_err(|e| anyhow::anyhow!("Failed to read response data: {}", e))?;

            Ok(response_data)
        }
        
        ironposh_client_core::connector::NetworkProtocol::Udp => {
            // UDP implementation for Kerberos KDC communication
            let host = packet.url.host_str()
                .ok_or_else(|| anyhow::anyhow!("Missing host in URL"))?;
            let port = packet.url.port()
                .ok_or_else(|| anyhow::anyhow!("Missing port in URL"))?;

            // Create UDP socket
            let socket = UdpSocket::bind("0.0.0.0:0")
                .map_err(|e| anyhow::anyhow!("Failed to bind UDP socket: {}", e))?;

            // Set timeout for UDP operations
            socket.set_read_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| anyhow::anyhow!("Failed to set UDP read timeout: {}", e))?;
            socket.set_write_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| anyhow::anyhow!("Failed to set UDP write timeout: {}", e))?;

            // Connect to the KDC
            socket.connect((host, port))
                .map_err(|e| anyhow::anyhow!("Failed to connect UDP socket to {}:{}: {}", host, port, e))?;

            // For UDP, send packet data directly (no length prefix like TCP)
            socket.send(&packet.data)
                .map_err(|e| anyhow::anyhow!("Failed to send UDP packet: {}", e))?;

            // Read the response
            let mut response_data = vec![0u8; 65536]; // Max UDP packet size
            let bytes_received = socket.recv(&mut response_data)
                .map_err(|e| anyhow::anyhow!("Failed to receive UDP response: {}", e))?;

            response_data.truncate(bytes_received);
            Ok(response_data)
        }
        
        ironposh_client_core::connector::NetworkProtocol::Http => {
            // HTTP implementation for potential web-based authentication
            let request = ureq::post(packet.url.as_str())
                .set("Content-Type", "application/octet-stream")
                .set("User-Agent", "ironposh-client/1.0");

            let response = request.send_bytes(&packet.data)
                .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

            let mut response_data = Vec::new();
            response.into_reader().read_to_end(&mut response_data)
                .map_err(|e| anyhow::anyhow!("Failed to read HTTP response: {}", e))?;

            Ok(response_data)
        }
        
        ironposh_client_core::connector::NetworkProtocol::Https => {
            // HTTPS implementation for secure web-based authentication
            let request = ureq::post(packet.url.as_str())
                .set("Content-Type", "application/octet-stream")
                .set("User-Agent", "ironposh-client/1.0");

            let response = request.send_bytes(&packet.data)
                .map_err(|e| anyhow::anyhow!("HTTPS request failed: {}", e))?;

            let mut response_data = Vec::new();
            response.into_reader().read_to_end(&mut response_data)
                .map_err(|e| anyhow::anyhow!("Failed to read HTTPS response: {}", e))?;

            Ok(response_data)
        }
    }
}
