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
use tracing::{Instrument, Level, info, info_span, span};

use crate::{HostIo, HostSubmitter, HttpClient, session, session_serial};

/// Run the connector handshake loop: step through authentication until Connected.
async fn run_handshake<C: HttpClient>(
    config: WinRmConfig,
    client: &C,
    session_event_tx: &mpsc::UnboundedSender<crate::SessionEvent>,
) -> anyhow::Result<(
    Box<ironposh_client_core::connector::active_session::ActiveSession>,
    ironposh_client_core::connector::conntion_pool::TrySend,
)> {
    let mut connector = Connector::new(config);
    info!("Created connector, starting connection handshake...");

    let mut response = None;

    loop {
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
                return Ok((active_session, next_receive_request));
            }
        }
    }
}

/// Build the pipeline multiplexer task that routes events between user input and server output.
fn build_pipeline_multiplexer(
    mut user_input_tx: mpsc::Sender<UserOperation>,
    mut server_output_rx: mpsc::Receiver<UserEvent>,
    mut pipeline_input_rx: mpsc::Receiver<PipelineInput>,
    span_prefix: &'static str,
) -> impl std::future::Future<Output = anyhow::Result<()>> {
    let pipeline_map = Arc::new(futures::lock::Mutex::new(
        std::collections::HashMap::<uuid::Uuid, mpsc::Sender<UserEvent>>::new(),
    ));

    let pipeline_map_clone = Arc::clone(&pipeline_map);

    let server_span_name = if span_prefix == "Serial" {
        "SerialPipelineServerHandlerLoop"
    } else {
        "PipelineServerHandlerLoop"
    };
    let user_span_name = if span_prefix == "Serial" {
        "SerialPipelineInputHandlerLoop"
    } else {
        "PipelineInputHandlerLoop"
    };

    async move {
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
        }
        .instrument(span!(Level::INFO, "PipelineServerLoop", prefix = server_span_name));

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
        .instrument(span!(Level::INFO, "PipelineInputLoop", prefix = user_span_name));

        let (x, y) = join!(from_server, from_user);
        x.and(y)
    }
}

/// Establish connection and return client handle with background task (parallel mode).
pub fn establish_connection<C>(
    config: WinRmConfig,
    client: C,
) -> (
    ConnectionHandle,
    HostIo,
    mpsc::UnboundedReceiver<crate::SessionEvent>,
    impl std::future::Future<Output = anyhow::Result<()>>,
)
where
    C: HttpClient + 'static,
{
    let (user_input_tx, user_input_rx) = mpsc::channel(10);
    let (server_output_tx, server_output_rx) = mpsc::channel(10);
    let (host_call_tx, host_call_rx) = mpsc::unbounded();
    let (host_resp_tx, host_resp_rx) = mpsc::unbounded();
    let (session_event_tx, session_event_rx) = mpsc::unbounded();
    let session_event_tx_2 = session_event_tx.clone();

    let host_io = HostIo {
        host_call_rx,
        submitter: HostSubmitter(host_resp_tx),
    };

    let user_input_tx_clone = user_input_tx.clone();
    let active_session_task = async move {
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionStarted);

        let (active_session, next_request) =
            run_handshake(config, &client, &session_event_tx).await?;

        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionEstablished);
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ActiveSessionStarted);
        info!("Connection established, entering parallel session loop");

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
            Ok(()) => {
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

    let (pipeline_input_tx, pipeline_input_rx) = mpsc::channel(100);
    let multiplex_pipeline_task =
        build_pipeline_multiplexer(user_input_tx, server_output_rx, pipeline_input_rx, "Parallel");

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

/// Establish connection using the serial (single-connection) session loop.
///
/// All WinRM operations are serialized through a single HTTP connection,
/// required when the transport (e.g. Devolutions Gateway) only allows a
/// single WebSocket per token.
pub fn establish_connection_serial<C>(
    config: WinRmConfig,
    client: C,
) -> (
    ConnectionHandle,
    HostIo,
    mpsc::UnboundedReceiver<crate::SessionEvent>,
    impl std::future::Future<Output = anyhow::Result<()>>,
)
where
    C: HttpClient + 'static,
{
    let (user_input_tx, user_input_rx) = mpsc::channel(10);
    let (server_output_tx, server_output_rx) = mpsc::channel(10);
    let (host_call_tx, host_call_rx) = mpsc::unbounded();
    let (host_resp_tx, host_resp_rx) = mpsc::unbounded();
    let (session_event_tx, session_event_rx) = mpsc::unbounded();
    let session_event_tx_2 = session_event_tx.clone();

    let host_io = HostIo {
        host_call_rx,
        submitter: HostSubmitter(host_resp_tx),
    };

    let active_session_task = async move {
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionStarted);

        let (active_session, next_request) =
            run_handshake(config, &client, &session_event_tx).await?;

        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ConnectionEstablished);
        let _ = session_event_tx.unbounded_send(crate::SessionEvent::ActiveSessionStarted);
        info!("Serial connection established, entering serial session loop");

        let result = session_serial::start_serial_session_loop(
            next_request,
            *active_session,
            client,
            user_input_rx,
            server_output_tx,
            host_call_tx,
            host_resp_rx,
        )
        .instrument(info_span!("SerialActiveSession"))
        .await;

        match result {
            Ok(()) => {
                info!("Serial session loop ended");
                let _ = session_event_tx.unbounded_send(crate::SessionEvent::ActiveSessionEnded);
                Ok(())
            }
            Err(e) => {
                let _ = session_event_tx.unbounded_send(crate::SessionEvent::Error(e.to_string()));
                Err(e)
            }
        }
    }
    .instrument(info_span!("SerialMainTask"));

    let (pipeline_input_tx, pipeline_input_rx) = mpsc::channel(100);
    let multiplex_pipeline_task =
        build_pipeline_multiplexer(user_input_tx, server_output_rx, pipeline_input_rx, "Serial");

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
