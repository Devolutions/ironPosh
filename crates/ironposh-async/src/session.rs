use std::time::Duration;

use anyhow::Context;
use futures::channel::mpsc;
use futures::future::Either;
use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
use ironposh_client_core::connector::active_session::{TransportErrorDisposition, UserEvent};
use ironposh_client_core::connector::{
    ActiveSessionOutput, UserOperation,
    connection_pool::{ConnectionId, TrySend},
    http::HttpResponseTargeted,
};
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{HostResponse, HttpClient};

/// Resolve deferred send variants into concrete `SendBack` requests.
///
/// `SendAndThenReceive` and `PendingReceive` are resolved by calling `fire_receive()`
/// to build the actual Receive request, then returned as `SendBack` with all requests.
/// Other variants pass through unchanged.
fn resolve_deferred_sends(
    output: ActiveSessionOutput,
    active_session: &mut ironposh_client_core::connector::active_session::ActiveSession,
) -> anyhow::Result<ActiveSessionOutput> {
    match output {
        ActiveSessionOutput::SendAndThenReceive {
            send_request,
            then_receive_streams,
        } => {
            let recv = active_session
                .fire_receive(then_receive_streams)
                .context("Failed to build receive after send-then-receive")?;
            Ok(ActiveSessionOutput::SendBack(vec![send_request, recv]))
        }
        ActiveSessionOutput::PendingReceive { desired_streams } => {
            let recv = active_session
                .fire_receive(desired_streams)
                .context("Failed to build receive from PendingReceive")?;
            Ok(ActiveSessionOutput::SendBack(vec![recv]))
        }
        other => Ok(other),
    }
}

fn launch<C: HttpClient>(
    client: &C,
    try_send: TrySend,
) -> impl core::future::Future<Output = (ConnectionId, anyhow::Result<HttpResponseTargeted>)> {
    let conn_id = try_send.get_connection_id();
    let response = client.send_request(try_send);
    async move { (conn_id, response.await) }
}

/// Emit a `PoolLifecycleEvent` when the runspace pool state crossed a
/// disconnect/reconnect boundary since the last observation.
fn emit_pool_lifecycle_transition(
    prev_state: &mut ironposh_client_core::runspace_pool::RunspacePoolState,
    active_session: &ironposh_client_core::connector::active_session::ActiveSession,
    lifecycle_tx: &mpsc::UnboundedSender<crate::PoolLifecycleEvent>,
) {
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    let state = active_session.runspace_pool_state();
    if state == *prev_state {
        return;
    }

    match (*prev_state, state) {
        (RunspacePoolState::Connecting, RunspacePoolState::Disconnected) => {
            // The Reconnect request failed and was aborted by the active session.
            warn!(target: "session", shell_id = ?active_session.shell_id(), "reconnect failed; runspace pool stays disconnected");
            let _ = lifecycle_tx.unbounded_send(crate::PoolLifecycleEvent::ReconnectFailed {
                shell_id: active_session.shell_id(),
            });
        }
        (_, RunspacePoolState::Disconnected) => {
            info!(target: "session", shell_id = ?active_session.shell_id(), "runspace pool disconnected");
            let _ = lifecycle_tx.unbounded_send(crate::PoolLifecycleEvent::Disconnected {
                shell_id: active_session.shell_id(),
            });
        }
        (
            RunspacePoolState::Disconnected | RunspacePoolState::Connecting,
            RunspacePoolState::Opened,
        ) => {
            info!(target: "session", shell_id = ?active_session.shell_id(), "runspace pool reconnected");
            let _ = lifecycle_tx.unbounded_send(crate::PoolLifecycleEvent::Reconnected {
                shell_id: active_session.shell_id(),
            });
        }
        (RunspacePoolState::Disconnecting, RunspacePoolState::Opened) => {
            // The Disconnect request faulted and was aborted by the active session.
            warn!(target: "session", shell_id = ?active_session.shell_id(), "disconnect failed; runspace pool stays connected");
            let _ = lifecycle_tx.unbounded_send(crate::PoolLifecycleEvent::DisconnectFailed {
                shell_id: active_session.shell_id(),
            });
        }
        _ => {}
    }

    *prev_state = state;
}

/// Main active session loop that handles network responses and user operations
#[expect(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines)]
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
    lifecycle_tx: mpsc::UnboundedSender<crate::PoolLifecycleEvent>,
) -> anyhow::Result<()> {
    use ironposh_client_core::connector::active_session::ActiveSessionOutput;

    // pending HTTP requests
    let mut inflight: FuturesUnordered<_> = FuturesUnordered::new();

    // kick off the initial polling request
    inflight.push(launch(&client, runspace_polling_request));

    // Track the pool state to surface disconnect/reconnect transitions.
    let mut pool_state = active_session.runspace_pool_state();

    info!("Starting single-loop active session");

    enum LoopEvent {
        Http(Box<(ConnectionId, anyhow::Result<HttpResponseTargeted>)>),
        User(Box<Option<UserOperation>>),
    }

    // main single-threaded loop
    loop {
        let loop_event = {
            let http_next = if inflight.is_empty() {
                Either::Left(futures::future::pending::<(
                    ConnectionId,
                    anyhow::Result<HttpResponseTargeted>,
                )>())
            } else {
                Either::Right(inflight.select_next_some())
            };
            futures::pin_mut!(http_next);

            futures::select! {
                ready = http_next => LoopEvent::Http(Box::new(ready)),
                user_op = user_input_rx.next() => LoopEvent::User(Box::new(user_op)),
            }
        };

        match loop_event {
            // 1) any HTTP finishes
            LoopEvent::Http(ready) => {
                let (conn_id, result) = *ready;
                match result {
                    Ok(http_response) => {
                        trace!(
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

                        emit_pool_lifecycle_transition(
                            &mut pool_state,
                            &active_session,
                            &lifecycle_tx,
                        );

                        // Convert ActiveSessionOutput into new HTTPs / UI events
                        for out in step_results {
                            let out = resolve_deferred_sends(out, &mut active_session)?;
                            match out {
                                ActiveSessionOutput::Ignore => {}
                                ActiveSessionOutput::SendBack(reqs) => {
                                    trace!(target: "network", request_count = reqs.len(), "launching HTTP requests in parallel");
                                    for r in reqs {
                                        inflight.push(launch(&client, r));
                                    }
                                }
                                ActiveSessionOutput::SendBackError(e) => {
                                    error!(target: "session", error = %e, "session step failed");
                                    return Err(anyhow::anyhow!("Session step failed: {e}"));
                                }
                                ActiveSessionOutput::UserEvent(event) => {
                                    trace!(target: "user", event = ?event, "sending user event");
                                    if user_output_tx.send(event).await.is_err() {
                                        return Err(anyhow::anyhow!(
                                            "User output channel disconnected"
                                        ));
                                    }
                                }
                                ActiveSessionOutput::HostCall(host_call) => {
                                    debug!(host_call = ?host_call.method_name(), call_id = host_call.call_id(), scope = ?host_call.scope());

                                    if host_call_tx.unbounded_send(host_call).is_err() {
                                        return Err(anyhow::anyhow!("Host-call channel closed"));
                                    }

                                    let HostResponse {
                                        call_id,
                                        scope,
                                        submission,
                                    } = host_resp_rx.next().await.ok_or_else(|| {
                                        anyhow::anyhow!("Host-response channel closed")
                                    })?;

                                    let step_result = resolve_deferred_sends(
                                        active_session
                                            .accept_client_operation(
                                                UserOperation::SubmitHostResponse {
                                                    call_id,
                                                    scope,
                                                    submission,
                                                },
                                            )
                                            .context("Failed to submit host response")?,
                                        &mut active_session,
                                    )?;

                                    match step_result {
                                        ActiveSessionOutput::SendBack(reqs) => {
                                            for r in reqs {
                                                inflight.push(launch(&client, r));
                                            }
                                        }
                                        other => {
                                            process_session_outputs(
                                                vec![other],
                                                &mut user_output_tx,
                                                &mut user_input_tx,
                                                &host_call_tx,
                                                &mut host_resp_rx,
                                            )
                                            .await?;
                                        }
                                    }
                                }
                                ActiveSessionOutput::OperationSuccess => {
                                    trace!(target: "session", "operation completed successfully");
                                }
                                // Already resolved by resolve_deferred_sends
                                ActiveSessionOutput::SendAndThenReceive { .. }
                                | ActiveSessionOutput::PendingReceive { .. } => unreachable!(),
                            }
                        }
                    }
                    Err(e) => {
                        // A transport-level failure on a dying connection during
                        // disconnect/reconnect must not kill the whole session.
                        match active_session.handle_transport_error(conn_id) {
                            TransportErrorDisposition::Fatal => {
                                error!(target: "network", error = %e, "HTTP request failed");
                                return Err(anyhow::anyhow!("HTTP error: {e:#}"));
                            }
                            TransportErrorDisposition::Tolerated => {
                                warn!(
                                    target: "network",
                                    conn_id = conn_id.inner(),
                                    error = %e,
                                    "tolerating transport error on dying connection during disconnect"
                                );
                            }
                            TransportErrorDisposition::DisconnectAborted
                            | TransportErrorDisposition::ReconnectAborted => {
                                warn!(
                                    target: "network",
                                    conn_id = conn_id.inner(),
                                    error = %e,
                                    "transport error aborted the in-flight disconnect/reconnect"
                                );
                                emit_pool_lifecycle_transition(
                                    &mut pool_state,
                                    &active_session,
                                    &lifecycle_tx,
                                );
                            }
                        }
                    }
                }
            }

            // 2) user operations
            LoopEvent::User(user_op) => {
                debug!(target: "user", "processing user operation");
                if let Some(user_operation) = *user_op {
                    debug!(target: "user", operation = ?user_operation, "processing user operation");

                    let step_result = resolve_deferred_sends(
                        active_session
                            .accept_client_operation(user_operation)
                            .context("Failed to accept user operation")?,
                        &mut active_session,
                    )?;

                    // Track state changes driven by user operations (e.g. Opened →
                    // Disconnecting) so a later fault-driven revert is observable.
                    emit_pool_lifecycle_transition(&mut pool_state, &active_session, &lifecycle_tx);

                    match step_result {
                        ActiveSessionOutput::SendBack(reqs) => {
                            trace!(target: "network", request_count = reqs.len(), "launching HTTP requests from user operation");
                            for r in reqs {
                                inflight.push(launch(&client, r));
                            }
                        }
                        ActiveSessionOutput::UserEvent(event) => {
                            trace!(target: "user", event = ?event, "sending user event from user operation");
                            if user_output_tx.send(event).await.is_err() {
                                return Err(anyhow::anyhow!("User output channel disconnected"));
                            }
                        }
                        ActiveSessionOutput::HostCall(host_call) => {
                            debug!(host_call = ?host_call.method_name(), call_id = host_call.call_id(), scope = ?host_call.scope());

                            if host_call_tx.unbounded_send(host_call).is_err() {
                                return Err(anyhow::anyhow!("Host-call channel closed"));
                            }

                            let HostResponse {
                                call_id,
                                scope,
                                submission,
                            } = host_resp_rx
                                .next()
                                .await
                                .ok_or_else(|| anyhow::anyhow!("Host-response channel closed"))?;

                            let step_result = resolve_deferred_sends(
                                active_session
                                    .accept_client_operation(UserOperation::SubmitHostResponse {
                                        call_id,
                                        scope,
                                        submission,
                                    })
                                    .context("Failed to submit host response")?,
                                &mut active_session,
                            )?;

                            match step_result {
                                ActiveSessionOutput::SendBack(reqs) => {
                                    for r in reqs {
                                        inflight.push(launch(&client, r));
                                    }
                                }
                                other => {
                                    process_session_outputs(
                                        vec![other],
                                        &mut user_output_tx,
                                        &mut user_input_tx,
                                        &host_call_tx,
                                        &mut host_resp_rx,
                                    )
                                    .await?;
                                }
                            }
                        }
                        ActiveSessionOutput::OperationSuccess => {
                            trace!(target: "session", "operation completed successfully");
                        }
                        ActiveSessionOutput::SendBackError(e) => {
                            error!(target: "session", error = %e, "session step failed");
                            return Err(anyhow::anyhow!("Session step failed: {e}"));
                        }
                        ActiveSessionOutput::Ignore => {}
                        ActiveSessionOutput::SendAndThenReceive { .. }
                        | ActiveSessionOutput::PendingReceive { .. } => unreachable!(),
                    }
                } else {
                    info!("User input channel disconnected");
                    break; // UI side closed
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
            ActiveSessionOutput::SendBack(_)
            | ActiveSessionOutput::SendAndThenReceive { .. }
            | ActiveSessionOutput::PendingReceive { .. } => {
                // This should be handled at the caller level
                warn!(
                    "SendBack/SendAndThenReceive/PendingReceive should not reach process_session_outputs"
                );
            }
            ActiveSessionOutput::SendBackError(e) => {
                error!(target: "session", error = %e, "session step failed");
                return Err(anyhow::anyhow!("Session step failed: {e}"));
            }
            ActiveSessionOutput::UserEvent(event) => {
                trace!(target: "user", event = ?event, "sending user event");
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
                trace!(target: "session", "operation completed successfully");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fmt::Write as _,
        future::Future,
        pin::Pin,
        sync::mpsc as std_mpsc,
        task::{Context, Poll},
    };

    use anyhow::Context as AnyhowContext;
    use base64::Engine as _;
    use futures::channel::oneshot;
    use futures::task::noop_waker_ref;
    use ironposh_client_core::connector::{
        Connector, ConnectorStepResult, TransportSecurity, WinRmConfig,
        config::{AuthenticatorConfig, TlsOptions},
        connection_pool::{ConnectionId, TrySend},
        http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, ServerAddress},
    };
    use ironposh_psrp::{
        ApplicationPrivateData, Destination, HostDefaultData, HostInfo, PowerShellRemotingMessage,
        RunspacePoolStateMessage, RunspacePoolStateValue, SessionCapability, Size,
        fragmentation::Fragment, ps_value::PsObjectWithType,
    };

    #[derive(Debug, PartialEq, Eq)]
    enum RequestKind {
        Receive,
        Disconnect,
        Reconnect,
    }

    struct SentRequest {
        kind: RequestKind,
        conn_id: ConnectionId,
        responder: oneshot::Sender<anyhow::Result<HttpResponseTargeted>>,
    }

    #[derive(Clone)]
    struct ControlledHttpClient {
        sent_tx: std_mpsc::Sender<SentRequest>,
    }

    impl HttpClient for ControlledHttpClient {
        fn send_request(
            &self,
            try_send: TrySend,
        ) -> impl Future<Output = anyhow::Result<HttpResponseTargeted>> {
            let sent_tx = self.sent_tx.clone();
            async move {
                let (request, conn_id) = expect_just_send(try_send);
                let kind = classify_request(&request);
                let (responder, response_rx) = oneshot::channel();
                sent_tx
                    .send(SentRequest {
                        kind,
                        conn_id,
                        responder,
                    })
                    .expect("test request receiver must stay alive");
                response_rx.await.context("test response sender dropped")?
            }
        }
    }

    #[test]
    fn parallel_loop_accepts_user_input_after_all_http_requests_drain() {
        let (active_session, initial_receive) = establish_active_session();
        let (sent_tx, sent_rx) = std_mpsc::channel();
        let client = ControlledHttpClient { sent_tx };
        let (mut user_input_tx, user_input_rx) = mpsc::channel(8);
        let (user_output_tx, _user_output_rx) = mpsc::channel(8);
        let (host_call_tx, _host_call_rx) = mpsc::unbounded();
        let (_host_resp_tx, host_resp_rx) = mpsc::unbounded();
        let (lifecycle_tx, _lifecycle_rx) = mpsc::unbounded();

        let session = start_active_session_loop(
            initial_receive,
            active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx.clone(),
            host_call_tx,
            host_resp_rx,
            lifecycle_tx,
        );
        futures::pin_mut!(session);

        assert_pending(poll_session(session.as_mut()));
        let initial_receive = recv_request(&sent_rx);
        assert_eq!(initial_receive.kind, RequestKind::Receive);

        user_input_tx
            .try_send(UserOperation::Disconnect)
            .expect("send Disconnect operation");
        assert_pending(poll_session(session.as_mut()));
        let disconnect = recv_request(&sent_rx);
        assert_eq!(disconnect.kind, RequestKind::Disconnect);

        disconnect
            .responder
            .send(Ok(xml_response(
                disconnect.conn_id,
                shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
            )))
            .expect("complete Disconnect request");
        assert_pending(poll_session(session.as_mut()));

        initial_receive
            .responder
            .send(Ok(xml_response(
                initial_receive.conn_id,
                FAULT_ENVELOPE.to_owned(),
            )))
            .expect("complete stale receive request");
        assert_pending(poll_session(session.as_mut()));

        user_input_tx
            .try_send(UserOperation::Reconnect)
            .expect("send Reconnect operation after inflight drained");
        assert_pending(poll_session(session.as_mut()));
        let reconnect = recv_request(&sent_rx);
        assert_eq!(reconnect.kind, RequestKind::Reconnect);

        reconnect
            .responder
            .send(Ok(xml_response(
                reconnect.conn_id,
                shell_op_response_xml("ReconnectResponse", "<rsp:ReconnectResponse/>"),
            )))
            .expect("complete Reconnect request");
        assert_pending(poll_session(session.as_mut()));
        let post_reconnect_receive = recv_request(&sent_rx);
        assert_eq!(post_reconnect_receive.kind, RequestKind::Receive);
    }

    #[test]
    fn transport_error_on_dying_receive_is_tolerated_during_disconnect() {
        let (active_session, initial_receive) = establish_active_session();
        let (sent_tx, sent_rx) = std_mpsc::channel();
        let client = ControlledHttpClient { sent_tx };
        let (mut user_input_tx, user_input_rx) = mpsc::channel(8);
        let (user_output_tx, _user_output_rx) = mpsc::channel(8);
        let (host_call_tx, _host_call_rx) = mpsc::unbounded();
        let (_host_resp_tx, host_resp_rx) = mpsc::unbounded();
        let (lifecycle_tx, mut lifecycle_rx) = mpsc::unbounded();

        let session = start_active_session_loop(
            initial_receive,
            active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx.clone(),
            host_call_tx,
            host_resp_rx,
            lifecycle_tx,
        );
        futures::pin_mut!(session);

        assert_pending(poll_session(session.as_mut()));
        let dying_receive = recv_request(&sent_rx);
        assert_eq!(dying_receive.kind, RequestKind::Receive);

        user_input_tx
            .try_send(UserOperation::Disconnect)
            .expect("send Disconnect operation");
        assert_pending(poll_session(session.as_mut()));
        let disconnect = recv_request(&sent_rx);
        assert_eq!(disconnect.kind, RequestKind::Disconnect);

        // The long-poll Receive dies at the transport level (e.g. TCP reset)
        // while the Disconnect response is still pending.
        dying_receive
            .responder
            .send(Err(anyhow::anyhow!("connection reset by peer")))
            .expect("fail dying receive request");
        assert_pending(poll_session(session.as_mut()));

        disconnect
            .responder
            .send(Ok(xml_response(
                disconnect.conn_id,
                shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
            )))
            .expect("complete Disconnect request");
        assert_pending(poll_session(session.as_mut()));

        let event = lifecycle_rx
            .try_next()
            .expect("a lifecycle event must be emitted")
            .expect("lifecycle channel must stay open");
        assert!(
            matches!(event, crate::PoolLifecycleEvent::Disconnected { .. }),
            "expected Disconnected lifecycle event, got {event:?}"
        );
    }

    #[test]
    fn transport_error_on_disconnect_connection_aborts_disconnect() {
        let (active_session, initial_receive) = establish_active_session();
        let (sent_tx, sent_rx) = std_mpsc::channel();
        let client = ControlledHttpClient { sent_tx };
        let (mut user_input_tx, user_input_rx) = mpsc::channel(8);
        let (user_output_tx, _user_output_rx) = mpsc::channel(8);
        let (host_call_tx, _host_call_rx) = mpsc::unbounded();
        let (_host_resp_tx, host_resp_rx) = mpsc::unbounded();
        let (lifecycle_tx, mut lifecycle_rx) = mpsc::unbounded();

        let session = start_active_session_loop(
            initial_receive,
            active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx.clone(),
            host_call_tx,
            host_resp_rx,
            lifecycle_tx,
        );
        futures::pin_mut!(session);

        assert_pending(poll_session(session.as_mut()));
        let initial_receive = recv_request(&sent_rx);
        assert_eq!(initial_receive.kind, RequestKind::Receive);

        user_input_tx
            .try_send(UserOperation::Disconnect)
            .expect("send Disconnect operation");
        assert_pending(poll_session(session.as_mut()));
        let disconnect = recv_request(&sent_rx);
        assert_eq!(disconnect.kind, RequestKind::Disconnect);

        // The Disconnect request itself fails at the transport level.
        disconnect
            .responder
            .send(Err(anyhow::anyhow!("connection reset by peer")))
            .expect("fail Disconnect request");
        assert_pending(poll_session(session.as_mut()));

        let event = lifecycle_rx
            .try_next()
            .expect("a lifecycle event must be emitted")
            .expect("lifecycle channel must stay open");
        assert!(
            matches!(event, crate::PoolLifecycleEvent::DisconnectFailed { .. }),
            "expected DisconnectFailed lifecycle event, got {event:?}"
        );

        // The pool reverted to Opened: a subsequent user operation still works.
        user_input_tx
            .try_send(UserOperation::Disconnect)
            .expect("send Disconnect operation after aborted disconnect");
        assert_pending(poll_session(session.as_mut()));
        let retry = recv_request(&sent_rx);
        assert_eq!(retry.kind, RequestKind::Disconnect);
    }

    #[test]
    fn transport_error_on_reconnect_connection_aborts_reconnect() {
        let (active_session, initial_receive) = establish_active_session();
        let (sent_tx, sent_rx) = std_mpsc::channel();
        let client = ControlledHttpClient { sent_tx };
        let (mut user_input_tx, user_input_rx) = mpsc::channel(8);
        let (user_output_tx, _user_output_rx) = mpsc::channel(8);
        let (host_call_tx, _host_call_rx) = mpsc::unbounded();
        let (_host_resp_tx, host_resp_rx) = mpsc::unbounded();
        let (lifecycle_tx, mut lifecycle_rx) = mpsc::unbounded();

        let session = start_active_session_loop(
            initial_receive,
            active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx.clone(),
            host_call_tx,
            host_resp_rx,
            lifecycle_tx,
        );
        futures::pin_mut!(session);

        assert_pending(poll_session(session.as_mut()));
        let dying_receive = recv_request(&sent_rx);
        assert_eq!(dying_receive.kind, RequestKind::Receive);

        user_input_tx
            .try_send(UserOperation::Disconnect)
            .expect("send Disconnect operation");
        assert_pending(poll_session(session.as_mut()));
        let disconnect = recv_request(&sent_rx);
        assert_eq!(disconnect.kind, RequestKind::Disconnect);

        disconnect
            .responder
            .send(Ok(xml_response(
                disconnect.conn_id,
                shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
            )))
            .expect("complete Disconnect request");
        assert_pending(poll_session(session.as_mut()));

        let event = lifecycle_rx
            .try_next()
            .expect("a lifecycle event must be emitted")
            .expect("lifecycle channel must stay open");
        assert!(
            matches!(event, crate::PoolLifecycleEvent::Disconnected { .. }),
            "expected Disconnected lifecycle event, got {event:?}"
        );

        // The stale Receive dies at the transport level while disconnected.
        dying_receive
            .responder
            .send(Err(anyhow::anyhow!("connection reset by peer")))
            .expect("fail stale receive request");
        assert_pending(poll_session(session.as_mut()));

        user_input_tx
            .try_send(UserOperation::Reconnect)
            .expect("send Reconnect operation");
        assert_pending(poll_session(session.as_mut()));
        let reconnect = recv_request(&sent_rx);
        assert_eq!(reconnect.kind, RequestKind::Reconnect);

        // The Reconnect request itself fails at the transport level.
        reconnect
            .responder
            .send(Err(anyhow::anyhow!("connection reset by peer")))
            .expect("fail Reconnect request");
        assert_pending(poll_session(session.as_mut()));

        let event = lifecycle_rx
            .try_next()
            .expect("a lifecycle event must be emitted")
            .expect("lifecycle channel must stay open");
        assert!(
            matches!(event, crate::PoolLifecycleEvent::ReconnectFailed { .. }),
            "expected ReconnectFailed lifecycle event, got {event:?}"
        );

        // The pool reverted to Disconnected: a reconnect retry still works.
        user_input_tx
            .try_send(UserOperation::Reconnect)
            .expect("send Reconnect operation after aborted reconnect");
        assert_pending(poll_session(session.as_mut()));
        let retry = recv_request(&sent_rx);
        assert_eq!(retry.kind, RequestKind::Reconnect);
    }

    #[test]
    fn transport_error_in_normal_state_is_fatal() {
        let (active_session, initial_receive) = establish_active_session();
        let (sent_tx, sent_rx) = std_mpsc::channel();
        let client = ControlledHttpClient { sent_tx };
        let (user_input_tx, user_input_rx) = mpsc::channel(8);
        let (user_output_tx, _user_output_rx) = mpsc::channel(8);
        let (host_call_tx, _host_call_rx) = mpsc::unbounded();
        let (_host_resp_tx, host_resp_rx) = mpsc::unbounded();
        let (lifecycle_tx, _lifecycle_rx) = mpsc::unbounded();

        let session = start_active_session_loop(
            initial_receive,
            active_session,
            client,
            user_input_rx,
            user_output_tx,
            user_input_tx,
            host_call_tx,
            host_resp_rx,
            lifecycle_tx,
        );
        futures::pin_mut!(session);

        assert_pending(poll_session(session.as_mut()));
        let receive = recv_request(&sent_rx);
        assert_eq!(receive.kind, RequestKind::Receive);

        receive
            .responder
            .send(Err(anyhow::anyhow!("connection reset by peer")))
            .expect("fail Receive request");

        match poll_session(session.as_mut()) {
            Poll::Ready(Err(_)) => {}
            Poll::Ready(Ok(())) => panic!("session loop ended without surfacing the error"),
            Poll::Pending => {
                panic!("transport error in Opened state must terminate the session loop")
            }
        }
    }

    fn poll_session<F>(future: Pin<&mut F>) -> Poll<anyhow::Result<()>>
    where
        F: Future<Output = anyhow::Result<()>>,
    {
        let mut cx = Context::from_waker(noop_waker_ref());
        future.poll(&mut cx)
    }

    fn assert_pending(result: Poll<anyhow::Result<()>>) {
        match result {
            Poll::Pending => {}
            Poll::Ready(Ok(())) => panic!("session loop ended unexpectedly"),
            Poll::Ready(Err(error)) => panic!("session loop failed unexpectedly: {error:#}"),
        }
    }

    fn recv_request(sent_rx: &std_mpsc::Receiver<SentRequest>) -> SentRequest {
        sent_rx
            .try_recv()
            .expect("session loop must have issued an HTTP request")
    }

    fn establish_active_session() -> (
        ironposh_client_core::connector::active_session::ActiveSession,
        TrySend,
    ) {
        let mut connector = Connector::new(test_config());

        let result = connector.step(None).expect("idle step");
        let ConnectorStepResult::SendBack { try_send } = result else {
            panic!("expected SendBack for Create");
        };
        let (request, conn_id) = expect_just_send(try_send);
        let create_xml = request
            .body
            .expect("create has a body")
            .as_str()
            .expect("plaintext body")
            .to_owned();
        let rpid = extract_shell_id(&create_xml);

        let create_response =
            include_str!("../../ironposh-client-core/tests/resources/resource_created.xml");
        let result = connector
            .step(Some(xml_response(conn_id, create_response.to_owned())))
            .expect("accept CreateResponse");
        let ConnectorStepResult::SendBack { try_send } = result else {
            panic!("expected SendBack for Receive");
        };
        let (_request, conn_id) = expect_just_send(try_send);

        let session_capability = SessionCapability {
            protocol_version: "2.3".to_owned(),
            ps_version: "2.0".to_owned(),
            serialization_version: "1.1.0.1".to_owned(),
            time_zone: None,
        };
        let application_private_data = ApplicationPrivateData::new();
        let pool_opened = RunspacePoolStateMessage::builder()
            .runspace_state(RunspacePoolStateValue::Opened)
            .build();
        let receive_response = receive_response_xml(
            rpid,
            &[&session_capability, &application_private_data, &pool_opened],
        );

        let result = connector
            .step(Some(xml_response(conn_id, receive_response)))
            .expect("accept ReceiveResponse");
        let ConnectorStepResult::Connected {
            active_session,
            send_this_one_async_or_you_stuck,
        } = result
        else {
            panic!("expected Connected, got {}", result.name());
        };

        (*active_session, send_this_one_async_or_you_stuck)
    }

    fn expect_just_send(try_send: TrySend) -> (HttpRequest, ConnectionId) {
        match try_send {
            TrySend::JustSend { request, conn_id } => (request, conn_id),
            TrySend::AuthNeeded { .. } => panic!("expected JustSend"),
        }
    }

    fn classify_request(request: &HttpRequest) -> RequestKind {
        let body = request
            .body
            .as_ref()
            .expect("test request has a body")
            .as_str()
            .expect("test request body is plaintext XML");

        if body.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect") {
            RequestKind::Disconnect
        } else if body.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Reconnect")
        {
            RequestKind::Reconnect
        } else if body.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Receive") {
            RequestKind::Receive
        } else {
            panic!("unexpected request body: {body}");
        }
    }

    fn test_config() -> WinRmConfig {
        let size = Size {
            width: 80,
            height: 25,
        };
        let host_data = HostDefaultData::builder()
            .buffer_size(size.clone())
            .window_size(size.clone())
            .max_window_size(size.clone())
            .max_physical_window_size(size)
            .build();
        let host_info = HostInfo::builder()
            .host_default_data(host_data)
            .use_runspace_host(true)
            .build();

        WinRmConfig {
            server: (ServerAddress::parse("127.0.0.1").unwrap(), 5985),
            transport: TransportSecurity::HttpInsecure,
            authentication: AuthenticatorConfig::Basic {
                username: "user".into(),
                password: "pass".into(),
            },
            host_info,
            operation_timeout_secs: Some(1.0),
            tls: TlsOptions::default(),
            configuration_name: None,
        }
    }

    fn extract_shell_id(create_xml: &str) -> uuid::Uuid {
        let marker = "ShellId=\"";
        let start = create_xml
            .find(marker)
            .map(|idx| idx + marker.len())
            .expect("Create request must carry a ShellId attribute");
        create_xml[start..start + 36]
            .parse()
            .expect("ShellId must be a UUID")
    }

    fn receive_response_xml(rpid: uuid::Uuid, messages: &[&dyn PsObjectWithType]) -> String {
        let mut streams = String::new();
        for (index, message) in messages.iter().enumerate() {
            let remoting_message = PowerShellRemotingMessage::new(
                Destination::Client,
                message.message_type(),
                rpid,
                None,
                &message.to_ps_object(),
            )
            .expect("serialize PSRP message");
            let fragment = Fragment::new(index as u64 + 1, 0, remoting_message.pack(), true, true);
            let payload = base64::engine::general_purpose::STANDARD.encode(fragment.pack());
            write!(
                streams,
                r#"<rsp:Stream Name="stdout">{payload}</rsp:Stream>"#
            )
            .expect("write stream XML");
        }

        format!(
            r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/ReceiveResponse</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <rsp:ReceiveResponse>{streams}</rsp:ReceiveResponse>
    </s:Body>
</s:Envelope>"#
        )
    }

    fn xml_response(conn_id: ConnectionId, xml: String) -> HttpResponseTargeted {
        HttpResponseTargeted::new(
            HttpResponse {
                status_code: 200,
                headers: vec![],
                body: HttpBody::Xml(xml),
            },
            conn_id,
            None,
        )
    }

    fn shell_op_response_xml(action: &str, body_element: &str) -> String {
        format!(
            r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/{action}</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        {body_element}
    </s:Body>
</s:Envelope>"#
        )
    }

    const FAULT_ENVELOPE: &str = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.dmtf.org/wbem/wsman/1/wsman/fault</a:Action>
        <a:MessageID>uuid:BB7AF8AE-D64A-422D-B36E-15A04FA17C5C</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>s:Sender</s:Value>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en-US">The shell is disconnected.</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#;
}
