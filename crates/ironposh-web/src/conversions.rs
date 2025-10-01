use std::convert::TryFrom;

use crate::types::{WasmPowerShellEvent, WasmWinRmConfig};
use ironposh_client_core::{
    connector::active_session::UserEvent,
    connector::{config::AuthenticatorConfig, http::ServerAddress, Scheme, WinRmConfig},
};
use ironposh_psrp::messages::init_runspace_pool::{HostDefaultData, HostInfo, Size};

// Convert WASM config to internal config
impl From<WasmWinRmConfig> for WinRmConfig {
    fn from(config: WasmWinRmConfig) -> Self {
        WinRmConfig {
            server: (
                ServerAddress::parse(&config.server).expect("Invalid server address"),
                config.port,
            ),
            scheme: if config.use_https {
                Scheme::Https
            } else {
                Scheme::Http
            },
            authentication: AuthenticatorConfig::Basic {
                username: config.username,
                password: config.password,
            },
            host_info: HostInfo::builder()
                .host_default_data(
                    HostDefaultData::builder()
                        .buffer_size(Size {
                            width: 120,
                            height: 30,
                        })
                        .window_size(Size {
                            width: 120,
                            height: 30,
                        })
                        .max_window_size(Size {
                            width: 120,
                            height: 30,
                        })
                        .max_physical_window_size(Size {
                            width: 120,
                            height: 30,
                        })
                        .build(),
                )
                .build(),
        }
    }
}

// Convert internal UserEvent to WASM event
impl TryFrom<&UserEvent> for WasmPowerShellEvent {
    type Error = crate::error::WasmError;
    fn try_from(value: &UserEvent) -> Result<Self, Self::Error> {
        let res = match value {
            UserEvent::PipelineCreated { pipeline } => WasmPowerShellEvent::PipelineCreated {
                pipeline_id: pipeline.id().to_string(),
            },
            UserEvent::PipelineFinished { pipeline } => WasmPowerShellEvent::PipelineFinished {
                pipeline_id: pipeline.id().to_string(),
            },
            UserEvent::PipelineOutput { pipeline, output } => WasmPowerShellEvent::PipelineOutput {
                pipeline_id: pipeline.id().to_string(),
                data: output.format_as_displyable_string()?,
            },
            UserEvent::ErrorRecord {
                error_record,
                handle,
            } => WasmPowerShellEvent::PipelineError {
                pipeline_id: handle.id().to_string(),
                error: format!("{:?}", error_record),
            },
        };

        Ok(res)
    }
}
