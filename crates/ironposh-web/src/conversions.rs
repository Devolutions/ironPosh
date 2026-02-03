use std::convert::TryFrom;

use crate::{
    error::WasmError,
    types::{
        GatewayTransport, SecurityWarning, WasmAuthMethod, WasmHostInformationMessage,
        WasmInformationMessageData, WasmPowerShellEvent, WasmPsrpRecord, WasmPsrpRecordMeta,
        WasmWinRmConfig,
    },
    WasmErrorRecord,
};
use ironposh_client_core::{
    connector::active_session::UserEvent,
    connector::{
        config::{AuthenticatorConfig, KerberosConfig, SspiAuthConfig},
        http::ServerAddress,
        TransportSecurity, WinRmConfig,
    },
    credentials::{ClientAuthIdentity, ClientUserName},
};
use ironposh_psrp::messages::init_runspace_pool::{HostDefaultData, HostInfo, Size};
use tracing::warn;

// =============================================================================
// Security Check
// =============================================================================

impl WasmWinRmConfig {
    /// Check the configuration for security issues and return any warnings.
    /// Returns an empty vec if the configuration is secure.
    ///
    /// Security model:
    /// - SSPI (TCP + !force_insecure) provides END-TO-END encryption.
    ///   Data is encrypted in the browser and decrypted only at the target server.
    ///   Gateway just forwards encrypted bytes, so gateway channel security doesn't matter.
    /// - TLS provides encryption for the destination channel only.
    ///   Gateway channel security matters in this case.
    pub fn check_security(&self) -> Vec<SecurityWarning> {
        let gateway_secure = self.gateway_url.starts_with("wss://");

        // Check if SSPI sealing is enabled (TCP transport without force_insecure)
        let sspi_enabled = matches!(self.destination.transport, GatewayTransport::Tcp)
            && !self.force_insecure.unwrap_or(false);

        // SSPI is end-to-end encryption - if enabled, data is always secure regardless of gateway
        if sspi_enabled {
            return vec![]; // End-to-end SSPI encryption - always secure
        }

        // No SSPI - check both channels
        let destination_secure = matches!(self.destination.transport, GatewayTransport::Tls);

        match (gateway_secure, destination_secure) {
            (true, true) => vec![],                                         // WSS + TLS
            (false, false) => vec![SecurityWarning::BothChannelsInsecure],  // WS + TCP without SSPI
            (false, true) => vec![SecurityWarning::GatewayChannelInsecure], // WS + TLS (gateway exposed)
            (true, false) => vec![SecurityWarning::DestinationChannelInsecure], // WSS + TCP without SSPI
        }
    }
}

// =============================================================================
// Config Conversion
// =============================================================================

impl From<WasmWinRmConfig> for WinRmConfig {
    fn from(config: WasmWinRmConfig) -> Self {
        let WasmWinRmConfig {
            auth,
            destination,
            username,
            password,
            domain,
            locale: _,
            gateway_url: _,
            gateway_token: _,
            kdc_proxy_url,
            client_computer_name,
            cols,
            rows,
            force_insecure,
        } = config;

        let size = Size {
            width: cols as i32,
            height: rows as i32,
        };

        let server =
            ServerAddress::parse(&destination.host).expect("Invalid destination host address");

        // Determine transport security based on gateway transport mode:
        // - TLS: Gateway wraps connection in TLS → SSPI sealing OFF (TLS provides encryption)
        // - TCP: Gateway uses plain TCP → SSPI sealing ON (unless force_insecure)
        let transport = match destination.transport {
            GatewayTransport::Tls => {
                // TLS provides encryption, SSPI sealing not needed
                TransportSecurity::Https
            }
            GatewayTransport::Tcp => {
                if force_insecure.unwrap_or(false) {
                    // User explicitly disabled SSPI sealing - DANGEROUS!
                    warn!("SSPI encryption disabled on TCP transport - connection is INSECURE!");
                    TransportSecurity::HttpInsecure
                } else {
                    // Default: SSPI sealing enabled
                    TransportSecurity::Http
                }
            }
        };

        let host_info = HostInfo::builder()
            .host_default_data(
                HostDefaultData::builder()
                    .buffer_size(size.clone())
                    .window_size(size.clone())
                    .max_window_size(size.clone())
                    .max_physical_window_size(size)
                    .build(),
            )
            .build();

        let domain = domain.as_deref();
        let authentication = match auth {
            WasmAuthMethod::Basic => AuthenticatorConfig::Basic { username, password },
            WasmAuthMethod::Ntlm => {
                let client_username =
                    ClientUserName::new(&username, domain).expect("Invalid username/domain");
                let identity = ClientAuthIdentity::new(client_username, password);

                AuthenticatorConfig::Sspi(SspiAuthConfig::NTLM {
                    target: destination.host.clone(),
                    identity,
                })
            }
            WasmAuthMethod::Kerberos => {
                let client_username =
                    ClientUserName::new(&username, domain).expect("Invalid username/domain");
                let identity = ClientAuthIdentity::new(client_username, password);

                let kdc_url = kdc_proxy_url
                    .as_ref()
                    .map(|url| url.parse().expect("Invalid kdc_proxy_url"));

                AuthenticatorConfig::Sspi(SspiAuthConfig::Kerberos {
                    target: destination.host.clone(),
                    identity,
                    kerberos_config: KerberosConfig {
                        kdc_url,
                        client_computer_name: client_computer_name
                            .unwrap_or_else(|| destination.host.clone()),
                    },
                })
            }
            WasmAuthMethod::Negotiate => {
                let client_username =
                    ClientUserName::new(&username, domain).expect("Invalid username/domain");
                let identity = ClientAuthIdentity::new(client_username, password);

                let kdc_url = kdc_proxy_url
                    .as_ref()
                    .map(|url| url.parse().expect("Invalid kdc_proxy_url"));

                AuthenticatorConfig::Sspi(SspiAuthConfig::Negotiate {
                    target: destination.host.clone(),
                    identity,
                    kerberos_config: Some(KerberosConfig {
                        kdc_url,
                        client_computer_name: client_computer_name
                            .unwrap_or_else(|| destination.host.clone()),
                    }),
                })
            }
        };

        Self {
            server: (server, destination.port),
            transport,
            authentication,
            host_info,
        }
    }
}

// Convert internal UserEvent to WASM event
impl TryFrom<&UserEvent> for WasmPowerShellEvent {
    type Error = crate::error::WasmError;
    fn try_from(value: &UserEvent) -> Result<Self, Self::Error> {
        let res = match value {
            UserEvent::PipelineCreated { pipeline } => Self::PipelineCreated {
                pipeline_id: pipeline.id().to_string(),
            },
            UserEvent::PipelineFinished { pipeline } => Self::PipelineFinished {
                pipeline_id: pipeline.id().to_string(),
            },
            UserEvent::PipelineOutput { pipeline, output } => Self::PipelineOutput {
                pipeline_id: pipeline.id().to_string(),
                data: if let Ok(str) = output.assume_primitive_string() {
                    str.clone()
                } else {
                    warn!("Pipeline output is not a primitive string, attempting to format as displayable string");
                    let res = output.format_as_displyable_string().map_err(|e| {
                        WasmError::Generic(format!(
                            "{e}, failed to format Pipeline output as string"
                        ))
                    })?;

                    res
                },
            },
            UserEvent::ErrorRecord {
                error_record,
                handle,
            } => Self::PipelineError {
                pipeline_id: handle.id().to_string(),
                error: WasmErrorRecord::from(error_record),
            },
            UserEvent::PipelineRecord { pipeline, record } => {
                let meta = match record {
                    ironposh_client_core::psrp_record::PsrpRecord::Debug { meta, .. }
                    | ironposh_client_core::psrp_record::PsrpRecord::Verbose { meta, .. }
                    | ironposh_client_core::psrp_record::PsrpRecord::Warning { meta, .. }
                    | ironposh_client_core::psrp_record::PsrpRecord::Information { meta, .. }
                    | ironposh_client_core::psrp_record::PsrpRecord::Progress { meta, .. }
                    | ironposh_client_core::psrp_record::PsrpRecord::Unsupported { meta, .. } => {
                        meta
                    }
                };

                let meta = WasmPsrpRecordMeta {
                    message_type: format!("{:?}", meta.message_type),
                    message_type_value: meta.message_type_value,
                    stream: meta.stream.clone(),
                    command_id: meta.command_id.map(|id| id.to_string()),
                    data_len: meta.data_len,
                };

                let record: WasmPsrpRecord = match record {
                    ironposh_client_core::psrp_record::PsrpRecord::Debug { message, .. } => {
                        WasmPsrpRecord::Debug {
                            meta,
                            message: message.clone(),
                        }
                    }
                    ironposh_client_core::psrp_record::PsrpRecord::Verbose { message, .. } => {
                        WasmPsrpRecord::Verbose {
                            meta,
                            message: message.clone(),
                        }
                    }
                    ironposh_client_core::psrp_record::PsrpRecord::Warning { message, .. } => {
                        WasmPsrpRecord::Warning {
                            meta,
                            message: message.clone(),
                        }
                    }
                    ironposh_client_core::psrp_record::PsrpRecord::Information {
                        record, ..
                    } => {
                        let message_data = match &record.message_data {
                            ironposh_psrp::InformationMessageData::String(s) => {
                                WasmInformationMessageData::String { value: s.clone() }
                            }
                            ironposh_psrp::InformationMessageData::HostInformationMessage(m) => {
                                WasmInformationMessageData::HostInformationMessage {
                                    value: WasmHostInformationMessage {
                                        message: m.message.clone(),
                                        foreground_color: m.foreground_color,
                                        background_color: m.background_color,
                                        no_new_line: m.no_new_line,
                                    },
                                }
                            }
                            ironposh_psrp::InformationMessageData::Object(v) => {
                                WasmInformationMessageData::Object {
                                    value: crate::JsPsValue::from(v.clone()),
                                }
                            }
                        };
                        WasmPsrpRecord::Information {
                            meta,
                            message_data,
                            source: record.source.clone(),
                            time_generated: record.time_generated.clone(),
                            tags: record.tags.clone(),
                            user: record.user.clone(),
                            computer: record.computer.clone(),
                            process_id: record.process_id,
                        }
                    }
                    ironposh_client_core::psrp_record::PsrpRecord::Progress { record, .. } => {
                        WasmPsrpRecord::Progress {
                            meta,
                            activity: record.activity.clone(),
                            activity_id: record.activity_id,
                            status_description: record.status_description.clone(),
                            current_operation: record.current_operation.clone(),
                            parent_activity_id: record.parent_activity_id,
                            percent_complete: record.percent_complete,
                            seconds_remaining: record.seconds_remaining,
                        }
                    }
                    ironposh_client_core::psrp_record::PsrpRecord::Unsupported {
                        data_preview,
                        ..
                    } => WasmPsrpRecord::Unsupported {
                        meta,
                        data_preview: data_preview.clone(),
                    },
                };

                Self::PipelineRecord {
                    pipeline_id: pipeline.id().to_string(),
                    record,
                }
            }
        };

        Ok(res)
    }
}
