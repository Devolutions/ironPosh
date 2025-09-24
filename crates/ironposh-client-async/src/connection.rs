use anyhow::Context;
use futures::channel::mpsc;
use ironposh_client_core::connector::{
    Connector, ConnectorStepResult, UserOperation, WinRmConfig, active_session::UserEvent,
};
use tracing::{Instrument, info, info_span};

use crate::{HttpClient, session};

/// Establish connection and return client handle with background task
///
/// This function creates the connection channels and establishes a WinRM connection,
/// then starts the active session loop in the background.
pub fn establish_connection<C: HttpClient>(
    config: WinRmConfig,
    client: C,
) -> (
    ConnectionHandle,
    impl std::future::Future<Output = anyhow::Result<()>>,
)
where
    C: 'static,
{
    let (user_input_tx, user_input_rx) = mpsc::channel(10);
    let (user_output_tx, user_output_rx) = mpsc::channel(10);

    let user_input_tx_clone = user_input_tx.clone();
    let task = async move {
        let mut connector = Connector::new(config);
        info!("Created connector, starting connection...");

        let mut response = None;

        let (active_session, next_request) = loop {
            let step_result = connector
                .step(response.take())
                .context("Failed to step through connector")?;

            info!(step_result = ?step_result.name(), "Processing step result");

            match step_result {
                ConnectorStepResult::SendBack { try_send } => {
                    // Make the HTTP request
                    response = Some(client.send_request(try_send).await?);
                }
                ConnectorStepResult::Connected {
                    active_session,
                    send_this_one_async_or_you_stuck: next_receive_request,
                } => {
                    break (active_session, next_receive_request);
                }
            }
        };

        info!("Connection established, entering active session loop");
        session::start_active_session_loop(
            next_request,
            *active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx_clone,
        )
        .instrument(info_span!("ActiveSession"))
        .await?;

        info!("Active session loop ended");

        Ok(())
    }
    .instrument(info_span!("MainTask"));

    (
        ConnectionHandle {
            user_input_tx,
            user_output_rx,
            message_cache: std::collections::HashMap::new(),
        },
        task,
    )
}

/// Handle for communicating with the established connection
pub struct ConnectionHandle {
    pub user_input_tx: mpsc::Sender<UserOperation>,
    pub user_output_rx: mpsc::Receiver<UserEvent>,
    pub message_cache: std::collections::HashMap<uuid::Uuid, Vec<UserEvent>>,
}
