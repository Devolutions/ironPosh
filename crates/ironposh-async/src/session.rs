use std::time::Duration;

use anyhow::Context;
use futures::channel::mpsc;
use futures::future::Either;
use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_client_core::connector::{
    ActiveSessionOutput, UserOperation, conntion_pool::TrySend, http::HttpResponseTargeted,
};
use tracing::{debug, error, info, instrument, warn};

use crate::{HostResponse, HttpClient};

fn launch<C: HttpClient>(
    client: &C,
    try_send: TrySend,
) -> impl core::future::Future<Output = anyhow::Result<HttpResponseTargeted>> {
    client.send_request(try_send)
}

/// Main active session loop that handles network responses and user operations
#[instrument(skip_all)]
pub async fn start_active_session_loop(
    runspace_polling_request: TrySend,
    mut active_session: ironposh_client_core::connector::active_session::ActiveSession,
    client: impl HttpClient,
    mut user_input_rx: mpsc::Receiver<UserOperation>,
    mut user_output_tx: mpsc::Sender<UserEvent>,
    mut user_input_tx: mpsc::Sender<UserOperation>,
    host_call_tx: mpsc::UnboundedSender<ironposh_client_core::host::HostCall>,
    mut host_resp_rx: mpsc::UnboundedReceiver<HostResponse>,
) -> anyhow::Result<()> {
    use ironposh_client_core::connector::active_session::ActiveSessionOutput;

    // pending HTTP requests
    let mut inflight: FuturesUnordered<_> = FuturesUnordered::new();

    // kick off the initial polling request
    inflight.push(launch(&client, runspace_polling_request));

    info!("Starting single-loop active session");

    // main single-threaded loop
    loop {
        futures::select! {
            // 1) any HTTP finishes
            ready = inflight.select_next_some() => {
                match ready {
                    Ok(http_response) => {
                        info!(
                            target: "network",
                            "processing successful network response"
                        );

                        // The http_response is already a HttpResponseTargeted from the client
                        let targeted_response = http_response;
                        let step_results = active_session
                            .accept_server_response(targeted_response)
                            .map_err(|e| {
                                error!(target: "network", error = %e, "failed to accept server response");
                                e
                            })
                            .context("Failed to accept server response")?;

                        // Convert ActiveSessionOutput into new HTTPs / UI events
                        for out in step_results {
                            match out {
                                ActiveSessionOutput::Ignore => {
                                    // Do nothing
                                }
                                ActiveSessionOutput::SendBack(reqs) => {
                                    info!(
                                        target: "network",
                                        request_count = reqs.len(),
                                        "launching HTTP requests in parallel"
                                    );
                                    // launch all new HTTPs in parallel
                                    for r in reqs {
                                        inflight.push(launch(&client, r));
                                    }
                                }
                                ActiveSessionOutput::SendBackError(e) => {
                                    error!(target: "session", error = %e, "session step failed");
                                    return Err(anyhow::anyhow!("Session step failed: {e}"));
                                }
                                ActiveSessionOutput::UserEvent(event) => {
                                    info!(target: "user", event = ?event, "sending user event");
                                    if user_output_tx.send(event).await.is_err() {
                                        return Err(anyhow::anyhow!("User output channel disconnected"));
                                    }
                                }
                                ActiveSessionOutput::HostCall(host_call) => {
                                    debug!(host_call = ?host_call.method_name(), call_id = host_call.call_id(), scope = ?host_call.scope());

                                    // Forward to consumer
                                    if host_call_tx.unbounded_send(host_call).is_err() {
                                        return Err(anyhow::anyhow!("Host-call channel closed"));
                                    }

                                    // Await the consumer's reply
                                    let HostResponse { call_id, scope, submission } = host_resp_rx.next().await
                                        .ok_or_else(|| anyhow::anyhow!("Host-response channel closed"))?;

                                    let step_result = active_session
                                        .accept_client_operation(
                                            UserOperation::SubmitHostResponse {
                                                call_id,
                                                scope,
                                                submission,
                                            },
                                        )
                                        .map_err(|e| {
                                            error!(target: "user", error = %e, "failed to submit host response");
                                            e
                                        })
                                        .context("Failed to submit host response")?;

                                    process_session_outputs(vec![step_result], &mut user_output_tx, &mut user_input_tx, &host_call_tx, &mut host_resp_rx).await?;
                                }
                                ActiveSessionOutput::OperationSuccess => {
                                    info!(target: "session", "operation completed successfully");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Any HTTP error terminates the session
                        error!(target: "network", error = %e, "HTTP request failed");
                        return Err(anyhow::anyhow!("HTTP error: {e:#}"));
                    }
                }
            }

            // 2) user operations
            user_op = user_input_rx.next() => {
                info!(target: "user", "processing user operation");
                match user_op {
                    Some(user_operation) => {
                        info!(target: "user", operation = ?user_operation, "processing user operation");

                        let step_result = active_session
                            .accept_client_operation(user_operation)
                            .map_err(|e| {
                                error!(target: "user", error = %e, "failed to accept user operation");
                                e
                            })
                            .context("Failed to accept user operation")?;

                        match step_result {
                            ActiveSessionOutput::SendBack(reqs) => {
                                info!(
                                    target: "network",
                                    request_count = reqs.len(),
                                    "launching HTTP requests from user operation"
                                );
                                for r in reqs {
                                    inflight.push(launch(&client, r));
                                }
                            }
                            ActiveSessionOutput::UserEvent(event) => {
                                info!(target: "user", event = ?event, "sending user event from user operation");
                                if user_output_tx.send(event).await.is_err() {
                                    return Err(anyhow::anyhow!("User output channel disconnected"));
                                }
                            }
                            ActiveSessionOutput::HostCall(host_call) => {
                                debug!(host_call = ?host_call.method_name(), call_id = host_call.call_id(), scope = ?host_call.scope());

                                // Forward to consumer
                                if host_call_tx.unbounded_send(host_call).is_err() {
                                    return Err(anyhow::anyhow!("Host-call channel closed"));
                                }

                                // Await the consumer's reply
                                let HostResponse { call_id, scope, submission } = host_resp_rx.next().await
                                    .ok_or_else(|| anyhow::anyhow!("Host-response channel closed"))?;

                                let step_result = active_session
                                    .accept_client_operation(
                                        UserOperation::SubmitHostResponse {
                                            call_id,
                                            scope,
                                            submission,
                                        },
                                    )
                                    .map_err(|e| {
                                        error!(target: "user", error = %e, "failed to submit host response");
                                        e
                                    })
                                    .context("Failed to submit host response")?;

                                process_session_outputs(vec![step_result], &mut user_output_tx, &mut user_input_tx, &host_call_tx, &mut host_resp_rx).await?;
                            }
                            ActiveSessionOutput::OperationSuccess => {
                                info!(target: "session", "operation completed successfully");
                            }
                            ActiveSessionOutput::SendBackError(e) => {
                                error!(target: "session", error = %e, "session step failed");
                                return Err(anyhow::anyhow!("Session step failed: {e}"));
                            },
                            ActiveSessionOutput::Ignore => {
                                // Do nothing
                            }
                        }

                    }
                    None => {
                        info!("User input channel disconnected");
                        break; // UI side closed
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process session outputs - handles user events and host calls recursively
#[instrument(skip_all)]
async fn process_session_outputs(
    step_results: Vec<ActiveSessionOutput>,
    user_output_tx: &mut mpsc::Sender<UserEvent>,
    user_input_tx: &mut mpsc::Sender<UserOperation>,
    host_call_tx: &mpsc::UnboundedSender<ironposh_client_core::host::HostCall>,
    host_resp_rx: &mut mpsc::UnboundedReceiver<HostResponse>,
) -> anyhow::Result<()> {
    for step_result in step_results {
        match step_result {
            ActiveSessionOutput::Ignore => {
                // Do nothing
            }
            ActiveSessionOutput::SendBack(_) => {
                // This should be handled at the caller level
                warn!("SendBack should not reach process_session_outputs");
            }
            ActiveSessionOutput::SendBackError(e) => {
                error!(target: "session", error = %e, "session step failed");
                return Err(anyhow::anyhow!("Session step failed: {e}"));
            }
            ActiveSessionOutput::UserEvent(event) => {
                info!(target: "user", event = ?event, "sending user event");
                if user_output_tx.send(event).await.is_err() {
                    return Err(anyhow::anyhow!("User output channel disconnected"));
                }
            }
            ActiveSessionOutput::HostCall(host_call) => {
                debug!(host_call = ?host_call.method_name(), call_id = host_call.call_id(), scope = ?host_call.scope());

                // Forward to consumer
                if host_call_tx.unbounded_send(host_call).is_err() {
                    return Err(anyhow::anyhow!("Host-call channel closed"));
                }

                let next = host_resp_rx.next();

                let Ok(next) = with_timeout(next, Duration::from_secs(5)).await else {
                    error!("Timed out waiting for host response");
                    continue;
                };

                // Await the consumer's reply
                let HostResponse {
                    call_id,
                    scope,
                    submission,
                } = next.ok_or_else(|| anyhow::anyhow!("Host-response channel closed"))?;

                if user_input_tx
                    .send(UserOperation::SubmitHostResponse {
                        call_id,
                        scope,
                        submission,
                    })
                    .await
                    .is_err()
                {
                    return Err(anyhow::anyhow!("User input channel disconnected"));
                }
            }
            ActiveSessionOutput::OperationSuccess => {
                info!(target: "session", "operation completed successfully");
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Timeout;

pub async fn with_timeout<F, T>(fut: F, dur: Duration) -> Result<T, Timeout>
where
    F: Future<Output = T>,
{
    let timeout = futures_timer::Delay::new(dur);

    futures::pin_mut!(timeout);
    futures::pin_mut!(fut);

    match futures::future::select(fut, timeout).await {
        Either::Left((val, _sleep)) => Ok(val),
        Either::Right((_unit, _fut)) => Err(Timeout),
    }
}
