use anyhow::Context;
use pwsh_core::connector::active_session::ActiveSession;
use pwsh_core::connector::{Connector, ConnectorConfig, ConnectorStepResult};
use tracing::{info, info_span, warn};

use crate::http_client::make_http_request;

/// Establish connection to the PowerShell remote server
pub fn establish_connection(
    config: ConnectorConfig,
) -> anyhow::Result<(
    ActiveSession,
    pwsh_core::connector::http::HttpRequest<String>,
)> {
    let mut connector = Connector::new(config);
    info!("Created connector, starting connection...");

    let mut response = None;
    let _span = info_span!("ConnectionLoop").entered();

    let (active_session, next_request) = loop {
        let step_result = connector
            .step(response.take())
            .context("Failed to step through connector")?;

        info!(step_result = ?step_result.name(), "Processing step result");

        match step_result {
            ConnectorStepResult::SendBack(http_request) => {
                // Make the HTTP request (using ureq for simplicity in example)
                response = Some(make_http_request(&http_request)?);
            }
            ConnectorStepResult::SendBackError(e) => {
                warn!("Connection step failed: {}", e);
                anyhow::bail!("Connection failed: {}", e);
            }
            ConnectorStepResult::Connected {
                active_session,
                next_receive_request,
            } => {
                break (active_session, next_receive_request);
            }
        }
    };

    drop(_span);
    Ok((active_session, next_request))
}