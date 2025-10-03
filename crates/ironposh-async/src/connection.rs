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
use tracing::{Instrument, Level, Span, info, info_span, span};

use crate::{HostIo, HostSubmitter, HttpClient, session};

/// Establish connection and return client handle with background task
///
/// This function creates the connection channels and establishes a WinRM connection,
/// then starts the active session loop in the background.
pub fn establish_connection<C: HttpClient>(
    config: WinRmConfig,
    client: C,
) -> (
    ConnectionHandle,
    HostIo,
    mpsc::UnboundedReceiver<crate::SessionEvent>,
    impl std::future::Future<Output = anyhow::Result<()>>,
)
where
    C: 'static,
{
    let (mut user_input_tx, user_input_rx) = mpsc::channel(10);
    let (server_output_tx, mut server_output_rx) = mpsc::channel(10);

    // Create host call channels upfront
    let (host_call_tx, host_call_rx) = mpsc::unbounded();
    let (host_resp_tx, host_resp_rx) = mpsc::unbounded();

    // Create session event channel
    let (session_event_tx, session_event_rx) = mpsc::unbounded();
    let session_event_tx_2 = session_event_tx.clone();

    // Create host I/O interface for the consumer
    let host_io = HostIo {
        host_call_rx,
        submitter: HostSubmitter(host_resp_tx),
    };

    let user_input_tx_clone = user_input_tx.clone();
    let active_session_task = async move {
        // Emit ConnectionStarted event
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionStarted);

        let mut connector = Connector::new(config);
        info!("Created connector, starting connection...");

        let mut response = None;

        let (active_session, next_request) = loop {
            let step_result = connector
                .step(response.take())
                .context("Failed to step through connector");

            let step_result = match step_result {
                Ok(result) => result,
                Err(e) => {
                    let _ =
                        session_event_tx.unbounded_send(crate::SessionEvent::Error(e.to_string()));
                    return Err(e);
                }
            };

            info!(step_result = ?step_result.name(), "Processing step result");

            match step_result {
                ConnectorStepResult::SendBack { try_send } => {
                    match client.send_request(try_send).await {
                        Ok(resp) => response = Some(resp),
                        Err(e) => {
                            let _ = session_event_tx
                                .unbounded_send(crate::SessionEvent::Error(e.to_string()));
                            return Err(e);
                        }
                    }
                }
                ConnectorStepResult::Connected {
                    active_session,
                    send_this_one_async_or_you_stuck: next_receive_request,
                } => {
                    break (active_session, next_receive_request);
                }
            }
        };

        // Emit ConnectionEstablished event
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionEstablished);
        info!("Connection established, entering active session loop");

        // Emit ActiveSessionStarted event
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ActiveSessionStarted);

        let result = session::start_active_session_loop(
            next_request,
            *active_session,
            client,
            user_input_rx,
            server_output_tx,
            user_input_tx_clone,
            host_call_tx,
            host_resp_rx,
        )
        .instrument(info_span!("ActiveSession"))
        .await;

        match result {
            Ok(_) => {
                info!("Active session loop ended");
                let _ = session_event_tx.unbounded_send(crate::SessionEvent::ActiveSessionEnded);
                Ok(())
            }
            Err(e) => {
                let _ = session_event_tx.unbounded_send(crate::SessionEvent::Error(e.to_string()));
                Err(e)
            }
        }
    }
    .instrument(info_span!("MainTask"));

    let (pipeline_input_tx, mut pipeline_input_rx) = mpsc::channel(100);
    let multiplex_pipeline_task = async move {
        let pipeline_map =
            std::sync::Arc::new(futures::lock::Mutex::new(std::collections::HashMap::<
                uuid::Uuid,
                mpsc::Sender<UserEvent>,
            >::new()));

        let pipeline_map_clone = Arc::clone(&pipeline_map);

        let from_server = async move {
            while let Some(server_output_event) = server_output_rx.next().await {
                info!(?server_output_event, "Received server output event");
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
        }.instrument(span!(Level::INFO,"PipelineServerHandlerLoop"));

        let pipeline_map = pipeline_map_clone;
        let from_user = async move {
            while let Some(input) = pipeline_input_rx.next().await {
                info!(?input, "Received pipeline input");
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
        }
        .instrument(span!(Level::INFO, "PipelineInputHanderLoop"));

        let (x, y) = join!(from_server, from_user);
        x.and(y)
    };

    let joined_task = async move {
        let res = join!(active_session_task, multiplex_pipeline_task);
        let _ = session_event_tx_2.unbounded_send(crate::SessionEvent::Closed);
        res.0.and(res.1)
    };

    (
        ConnectionHandle { pipeline_input_tx },
        host_io,
        session_event_rx,
        joined_task,
    )
}

/// Handle for communicating with the established connection
#[derive(Clone)]
pub struct ConnectionHandle {
    pub pipeline_input_tx: mpsc::Sender<PipelineInput>,
}

#[derive(Debug)]
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
