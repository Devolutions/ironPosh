use std::sync::Arc;

use anyhow::Context;
use futures::{SinkExt, StreamExt, channel::mpsc, join};
use ironposh_client_core::{
    connector::{
        Connector, ConnectorStepResult, UserOperation, WinRmConfig, active_session::UserEvent,
    },
    pipeline::PipelineSpec,
    powershell::PipelineHandle,
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
    let (mut user_input_tx, user_input_rx) = mpsc::channel(10);
    let (server_output_tx, mut server_output_rx) = mpsc::channel(10);

    let user_input_tx_clone = user_input_tx.clone();
    let active_session_task = async move {
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
            server_output_tx,
            user_input_tx_clone,
        )
        .instrument(info_span!("ActiveSession"))
        .await?;

        info!("Active session loop ended");

        Ok(())
    }
    .instrument(info_span!("MainTask"));

    let (pipeline_input_tx, mut pipeline_input_rx) = mpsc::channel(10);
    let multiplex_pipeline_task = async move {
        let pipeline_map =
            std::sync::Arc::new(futures::lock::Mutex::new(std::collections::HashMap::<
                uuid::Uuid,
                mpsc::Sender<UserEvent>,
            >::new()));

        let pipeline_map_clone = Arc::clone(&pipeline_map);

        let from_server = async move {
            while let Some(server_output_event) = server_output_rx.next().await {
                let uuid = server_output_event.pipeline_id();
                let mut map = pipeline_map.lock().await;
                if let Some(sender) = map.get_mut(&uuid) {
                    let close = matches!(server_output_event, UserEvent::PipelineFinished { .. });

                    if let Err(e) = sender.clone().send(server_output_event).await {
                        info!(%e, pipeline_id = %uuid, "Failed to forward event to pipeline stream");
                    }

                    if close {
                        info!(pipeline_id = %uuid, "Closing stream for finished pipeline");
                        sender.close_channel();
                    }
                } else {
                    info!(pipeline_id = %uuid, "No stream found for pipeline event");
                }
            }

            Ok::<(), anyhow::Error>(())
        };

        let pipeline_map = pipeline_map_clone;
        let from_user = async move {
            while let Some(input) = pipeline_input_rx.next().await {
                match input {
                    PipelineInput::Invoke {
                        uuid,
                        spec,
                        response_tx,
                    } => {
                        let op = UserOperation::InvokeWithSpec { uuid, spec };
                        info!(?op, "Received pipeline operation");

                        let mut map = pipeline_map.lock().await;
                        map.insert(uuid, response_tx);

                        user_input_tx
                            .send(op)
                            .await
                            .context("Failed to forward pipeline operation")?;
                    }
                    PipelineInput::Kill { pipeline_handle } => {
                        let op = UserOperation::KillPipeline {
                            pipeline: pipeline_handle,
                        };
                        info!(?op, "Received pipeline kill operation");

                        user_input_tx
                            .send(op)
                            .await
                            .context("Failed to forward KillPipeline operation")?;
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        };

        let (x, y) = join!(from_server, from_user);
        x.and(y)
    };

    let joined_task = async move { join!(active_session_task, multiplex_pipeline_task).0 };

    (ConnectionHandle { pipeline_input_tx }, joined_task)
}

/// Handle for communicating with the established connection
pub struct ConnectionHandle {
    pub pipeline_input_tx: mpsc::Sender<PipelineInput>,
}

pub enum PipelineInput {
    Invoke {
        uuid: uuid::Uuid,
        spec: PipelineSpec,
        response_tx: mpsc::Sender<UserEvent>,
    },
    Kill {
        pipeline_handle: PipelineHandle,
    },
}
