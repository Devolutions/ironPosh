use std::convert::TryFrom;

use crate::{
    error::WasmError,
    types::{WasmAuthMethod, WasmPowerShellEvent, WasmWinRmConfig},
    WasmErrorRecord,
};
use ironposh_client_core::{
    connector::active_session::UserEvent,
    connector::{
        config::{AuthenticatorConfig, KerberosConfig, SspiAuthConfig},
        http::ServerAddress,
        Scheme, WinRmConfig,
    },
    credentials::{ClientAuthIdentity, ClientUserName},
};
use ironposh_psrp::messages::init_runspace_pool::{HostDefaultData, HostInfo, Size};
use tracing::warn;

// Convert WASM config to internal config
impl From<WasmWinRmConfig> for WinRmConfig {
    fn from(config: WasmWinRmConfig) -> Self {
        let WasmWinRmConfig {
            auth,
            server,
            port,
            use_https,
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
        } = config;

        let size = Size {
            width: cols as i32,
            height: rows as i32,
        };

        let server = ServerAddress::parse(&server).expect("Invalid server address");
        let scheme = if use_https {
            Scheme::Https
        } else {
            Scheme::Http
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

                AuthenticatorConfig::Sspi {
                    sspi: SspiAuthConfig::NTLM {
                        target: server.to_string(),
                        identity,
                    },
                    require_encryption: true,
                }
            }
            WasmAuthMethod::Kerberos => {
                let client_username =
                    ClientUserName::new(&username, domain).expect("Invalid username/domain");
                let identity = ClientAuthIdentity::new(client_username, password);

                let kdc_url = kdc_proxy_url
                    .as_ref()
                    .map(|url| url.parse().expect("Invalid kdc_proxy_url"));

                AuthenticatorConfig::Sspi {
                    sspi: SspiAuthConfig::Kerberos {
                        target: server.to_string(),
                        identity,
                        kerberos_config: KerberosConfig {
                            kdc_url,
                            client_computer_name: client_computer_name
                                .unwrap_or_else(|| server.to_string()),
                        },
                    },
                    require_encryption: true,
                }
            }
            WasmAuthMethod::Negotiate => {
                let client_username =
                    ClientUserName::new(&username, domain).expect("Invalid username/domain");
                let identity = ClientAuthIdentity::new(client_username, password);

                let kdc_url = kdc_proxy_url
                    .as_ref()
                    .map(|url| url.parse().expect("Invalid kdc_proxy_url"));

                AuthenticatorConfig::Sspi {
                    sspi: SspiAuthConfig::Negotiate {
                        target: server.to_string(),
                        identity,
                        kerberos_config: Some(KerberosConfig {
                            kdc_url,
                            client_computer_name: client_computer_name
                                .unwrap_or_else(|| server.to_string()),
                        }),
                    },
                    require_encryption: true,
                }
            }
        };

        Self {
            server: (server, port),
            scheme,
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
        };

        Ok(res)
    }
}
