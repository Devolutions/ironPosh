use std::convert::TryFrom;

use crate::{
    error::WasmError,
    types::{WasmPowerShellEvent, WasmWinRmConfig},
};
use ironposh_client_core::{
    connector::active_session::UserEvent,
    connector::{config::AuthenticatorConfig, http::ServerAddress, Scheme, WinRmConfig},
};
use ironposh_psrp::messages::init_runspace_pool::{HostDefaultData, HostInfo, Size};
use tracing::warn;

// Convert WASM config to internal config
impl From<WasmWinRmConfig> for WinRmConfig {
    fn from(config: WasmWinRmConfig) -> Self {
        let size = Size {
            width: config.cols as i32,
            height: config.rows as i32,
        };

        Self {
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
                        .buffer_size(size.clone())
                        .window_size(size.clone())
                        .max_window_size(size.clone())
                        .max_physical_window_size(size)
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
                    let res = output
                        .format_as_displyable_string()
                        .map_err(|e| {
                            WasmError::Generic(format!(
                                "{e}, failed to format Pipeline output as string"
                            ))
                        })?
                        ;

                    res
                },
            },
            UserEvent::ErrorRecord {
                error_record,
                handle,
            } => Self::PipelineError {
                pipeline_id: handle.id().to_string(),
                error: format!("{error_record:?}"),
            },
        };

        Ok(res)
    }
}
