use std::collections::VecDeque;

use anyhow::Context;
use futures::channel::mpsc;
use futures::future::Either;
use futures::{FutureExt, SinkExt, StreamExt};
use ironposh_client_core::connector::active_session::{ActiveSession, UserEvent};
use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation, conntion_pool::TrySend};
use ironposh_client_core::host::HostCall;
use ironposh_client_core::runspace_pool::DesiredStream;
use tracing::{debug, error, info, instrument, trace};

use crate::{HostResponse, HttpClient};

/// Console diagnostic logging for WASM debugging. Uses web_sys::console::error_1
/// when the wasm-diag feature is enabled (shows in Playwright's [browser:error]).
#[cfg(all(feature = "wasm-diag", target_arch = "wasm32"))]
macro_rules! diag {
    ($($arg:tt)*) => {
        web_sys::console::error_1(&format!($($arg)*).into());
    };
}
#[cfg(not(all(feature = "wasm-diag", target_arch = "wasm32")))]
macro_rules! diag {
    ($($arg:tt)*) => {};
}

/// Serial active session loop using a flat `select!`-based event loop.
///
/// Unlike the old phased design (Phase 1→2→3→4), this loop uses `futures::select!`
/// with three event sources: in-flight HTTP response, user input, and host-call response.
///
/// **Core invariant:** At most one HTTP request is in flight at any time, and
/// `accept_client_operation` is only called when the connection is idle (no HTTP in-flight).
///
/// While an HTTP request is in-flight, user operations and host-call responses are
/// **buffered** (not processed) because `accept_client_operation` may call
/// `ConnectionPool::send()` which would allocate a second connection — violating
/// the single-connection constraint enforced by the Devolutions Gateway.
///
/// **Key difference from the old design:** `enqueue_output()` never sends HTTP requests
/// inline. It only pushes work items into queues. The promotion logic at the top of each
/// outer loop iteration decides what to send next.
///
/// **HostCall-aware promotion:** Receives are NOT sent while a HostCall is pending
/// (`host_call_active || !pending_host_calls.is_empty()`). This avoids unnecessary
/// timeout round-trips: the server blocks Receives while waiting for our HostCall response,
/// so sending a Receive before submitting the HostCall response would always time out.
#[expect(clippy::too_many_lines)]
#[instrument(skip_all)]
pub async fn start_serial_session_loop(
    first_receive: TrySend,
    mut active_session: ActiveSession,
    client: impl HttpClient,
    mut user_input_rx: mpsc::Receiver<UserOperation>,
    mut user_output_tx: mpsc::Sender<UserEvent>,
    host_call_tx: mpsc::UnboundedSender<HostCall>,
    mut host_resp_rx: mpsc::UnboundedReceiver<HostResponse>,
) -> anyhow::Result<()> {
    // Pending HTTP requests to send (Command, HostResponse, Signal, etc.)
    // Send operations always take priority over Receive polling.
    let mut work_queue: VecDeque<TrySend> = VecDeque::new();

    // Accumulated Receive streams. Merged from PendingReceive and SendAndThenReceive.
    // Only built into an actual TrySend (via `fire_receive`) when there's nothing in
    // `work_queue`, no HTTP request in flight, and no pending HostCalls.
    let mut deferred_streams: Vec<DesiredStream> = Vec::new();

    // HostCalls waiting to be dispatched to the JS/consumer side.
    let mut pending_host_calls: VecDeque<HostCall> = VecDeque::new();

    // Whether we're currently waiting for a host-call response from JS.
    let mut host_call_active: bool = false;

    // User operations buffered during HTTP wait. Processed when connection becomes idle.
    let mut pending_user_ops: VecDeque<UserOperation> = VecDeque::new();

    // Seed the work queue with the first Receive from the connector handshake.
    work_queue.push_back(first_receive);

    info!("Starting serial session loop (flat event loop, single-connection mode)");
    diag!("DIAG serial loop: started (flat event loop)");

    loop {
        // === Process ONE buffered user op — ONLY when work_queue is empty ===
        //
        // When work_queue is non-empty, at least one TrySend has already called
        // ConnectionPool::send() internally, moving the connection to Pending.
        // Calling accept_client_operation now would trigger another send() on
        // the Pending connection → allocating a second connection and breaking
        // the single-connection invariant.
        //
        // We must drain work_queue first (each item does an HTTP round-trip that
        // returns the connection to Idle) before processing any buffered user op.
        //
        // Only one op per iteration: accept_client_operation may call send()
        // itself, putting the connection back into Pending. The resulting
        // work_queue item will be sent on the next promotion, returning the
        // connection to Idle so the next buffered op can be processed.
        if work_queue.is_empty()
            && let Some(op) = pending_user_ops.pop_front()
        {
            diag!("DIAG process buffered: {}", op.operation_type());
            debug!(target: "serial", operation = op.operation_type(), "processing buffered user operation");
            let output = active_session
                .accept_client_operation(op)
                .context("Failed to accept buffered user operation")?;
            enqueue_output(
                output,
                &mut work_queue,
                &mut deferred_streams,
                &mut pending_host_calls,
                &mut user_output_tx,
            )
            .await?;
        }

        // Dispatch any queued host calls (channel send only, no HTTP).
        try_dispatch_next_host_call(&mut pending_host_calls, &mut host_call_active, &host_call_tx)?;

        // === PROMOTION: pick the next thing to send ===
        let http_future = if let Some(req) = work_queue.pop_front() {
            diag!(
                "DIAG promote: sending from work_queue ({} remaining)",
                work_queue.len()
            );
            trace!(target: "serial", remaining_work = work_queue.len(), "promoting work_queue item");
            Some(client.send_request(req).fuse())
        } else if !deferred_streams.is_empty()
            && !host_call_active
            && pending_host_calls.is_empty()
        {
            // Only build Receives when all HostCalls are resolved. The server blocks
            // Receives while waiting for our HostCall response, so sending one early
            // would always time out (adding an unnecessary OperationTimeout delay).
            //
            // WinRM Receive schema constraint: only ONE <rsp:DesiredStream> element is
            // allowed per Receive request. We pick a single stream — preferring pipeline-
            // specific streams (with CommandId) over runspace pool streams (without).
            //
            // When pipeline streams exist, DROP runspace pool streams entirely. In serial
            // mode, a runspace pool Receive with no pending data would block for the full
            // OperationTimeout (~15-20s), starving pipeline Receives and HostCall delivery.
            // The parallel loop avoids this by running them concurrently; we can't do that.
            let has_pipeline = deferred_streams.iter().any(|s| s.command_id().is_some());
            if has_pipeline {
                deferred_streams.retain(|s| s.command_id().is_some());
            }
            let stream = deferred_streams.remove(0);

            diag!(
                "DIAG promote: Receive for 1 stream (pipeline={}, {} deferred remaining)",
                stream.command_id().is_some(),
                deferred_streams.len()
            );
            trace!(
                target: "serial",
                ?stream,
                deferred_remaining = deferred_streams.len(),
                "promoting single stream to Receive"
            );
            let receive = active_session
                .fire_receive(vec![stream])
                .context("Failed to build Receive from deferred stream")?;
            Some(client.send_request(receive).fuse())
        } else {
            None
        };

        if let Some(http_future) = http_future {
            // Pin the HTTP future — it must survive across inner loop iterations.
            futures::pin_mut!(http_future);

            // Inner loop: keep selecting until the HTTP response arrives.
            // User ops and host-call responses are BUFFERED (not processed)
            // because accept_client_operation needs an idle connection.
            loop {
                let mut host_guard = if host_call_active {
                    Either::Left(host_resp_rx.next())
                } else {
                    Either::Right(futures::future::pending::<Option<HostResponse>>())
                };

                futures::select! {
                    resp = http_future => {
                        let resp = resp.context("Serial HTTP request failed")?;
                        diag!("DIAG select: HTTP response received");
                        trace!(target: "serial", "HTTP response received");

                        let outputs = match active_session.accept_server_response(resp) {
                            Ok(outputs) => outputs,
                            Err(e) => {
                                diag!("DIAG ERROR: accept_server_response failed: {:#}", e);
                                return Err(e).context("Failed to accept server response");
                            }
                        };

                        let output_types: Vec<&str> =
                            outputs.iter().map(output_type_name).collect();
                        diag!(
                            "DIAG select: {} outputs: {:?}",
                            outputs.len(),
                            output_types
                        );
                        trace!(
                            target: "serial",
                            output_count = outputs.len(),
                            ?output_types,
                            "processing server response outputs"
                        );

                        for output in outputs {
                            enqueue_output(
                                output,
                                &mut work_queue,
                                &mut deferred_streams,
                                &mut pending_host_calls,
                                &mut user_output_tx,
                            )
                            .await?;
                        }

                        // Drain any user ops that arrived while HTTP was in flight.
                        loop {
                            match user_input_rx.try_next() {
                                Ok(Some(op)) => {
                                    diag!("DIAG drain: collected {}", op.operation_type());
                                    pending_user_ops.push_back(op);
                                }
                                Ok(None) => {
                                    info!("User input channel closed during drain");
                                    return Ok(());
                                }
                                Err(_) => break,
                            }
                        }

                        // Break inner loop — HTTP done, go back to top for
                        // buffered op processing and next promotion.
                        break;
                    }

                    op = user_input_rx.next() => {
                        if let Some(op) = op {
                            diag!(
                                "DIAG select: buffering user op {} (HTTP in flight)",
                                op.operation_type()
                            );
                            debug!(
                                target: "serial",
                                operation = op.operation_type(),
                                "buffering user operation (HTTP in flight)"
                            );
                            pending_user_ops.push_back(op);
                            // Continue inner loop — HTTP future still alive.
                        } else {
                            info!("User input channel closed, ending serial session loop");
                            return Ok(());
                        }
                    }

                    hr = host_guard => {
                        if let Some(hr) = hr {
                            diag!(
                                "DIAG select: buffering host response call_id={} (HTTP in flight)",
                                hr.call_id
                            );
                            debug!(
                                target: "serial",
                                call_id = hr.call_id,
                                "buffering host-call response (HTTP in flight)"
                            );
                            // Convert to UserOperation and buffer for later processing.
                            pending_user_ops.push_back(UserOperation::SubmitHostResponse {
                                call_id: hr.call_id,
                                scope: hr.scope,
                                submission: hr.submission,
                            });
                            host_call_active = false;
                            // Can dispatch next HostCall to JS (just a channel send, no HTTP).
                            try_dispatch_next_host_call(
                                &mut pending_host_calls,
                                &mut host_call_active,
                                &host_call_tx,
                            )?;
                            // Continue inner loop — HTTP future still alive.
                        } else {
                            return Err(anyhow::anyhow!("Host-response channel closed"));
                        }
                    }
                }
            }
        } else {
            // No HTTP in flight and nothing to promote. Idle — wait for user op or host response.
            trace!(target: "serial", "idle: no pending work, waiting for user input or host response");
            diag!("DIAG idle: waiting for user input or host response");

            let mut host_guard = if host_call_active {
                Either::Left(host_resp_rx.next())
            } else {
                Either::Right(futures::future::pending::<Option<HostResponse>>())
            };

            futures::select! {
                op = user_input_rx.next() => {
                    if let Some(op) = op {
                        diag!("DIAG idle: user op {}", op.operation_type());
                        debug!(
                            target: "serial",
                            operation = op.operation_type(),
                            "user operation received while idle"
                        );
                        // Connection is idle — safe to process directly.
                        let output = active_session
                            .accept_client_operation(op)
                            .context("Failed to accept user operation (idle)")?;
                        enqueue_output(
                            output,
                            &mut work_queue,
                            &mut deferred_streams,
                            &mut pending_host_calls,
                            &mut user_output_tx,
                        )
                        .await?;
                        try_dispatch_next_host_call(
                            &mut pending_host_calls,
                            &mut host_call_active,
                            &host_call_tx,
                        )?;
                    } else {
                        info!("User input channel closed (idle), ending serial session loop");
                        return Ok(());
                    }
                }

                hr = host_guard => {
                    if let Some(hr) = hr {
                        diag!(
                            "DIAG idle: host response received call_id={}",
                            hr.call_id
                        );
                        debug!(
                            target: "serial",
                            call_id = hr.call_id,
                            "host-call response received while idle"
                        );
                        host_call_active = false;

                        // Connection is idle — safe to process directly.
                        let output = active_session
                            .accept_client_operation(UserOperation::SubmitHostResponse {
                                call_id: hr.call_id,
                                scope: hr.scope,
                                submission: hr.submission,
                            })
                            .context("Failed to submit host response (idle)")?;

                        enqueue_output(
                            output,
                            &mut work_queue,
                            &mut deferred_streams,
                            &mut pending_host_calls,
                            &mut user_output_tx,
                        )
                        .await?;

                        try_dispatch_next_host_call(
                            &mut pending_host_calls,
                            &mut host_call_active,
                            &host_call_tx,
                        )?;
                    } else {
                        return Err(anyhow::anyhow!("Host-response channel closed (idle)"));
                    }
                }
            }
        }
    }
}

/// Enqueue an `ActiveSessionOutput` into the appropriate queue. **Never sends HTTP inline.**
///
/// This is the key difference from the old `process_serial_output`: all network work is
/// pushed to `work_queue` or `deferred_streams` for the promotion logic to pick up on the
/// next outer loop iteration.
///
/// **Important:** This function does NOT dispatch HostCalls. The caller must call
/// `try_dispatch_next_host_call()` after processing all outputs to dispatch queued HostCalls.
async fn enqueue_output(
    output: ActiveSessionOutput,
    work_queue: &mut VecDeque<TrySend>,
    deferred_streams: &mut Vec<DesiredStream>,
    pending_host_calls: &mut VecDeque<HostCall>,
    user_output_tx: &mut mpsc::Sender<UserEvent>,
) -> anyhow::Result<()> {
    match output {
        ActiveSessionOutput::SendBack(reqs) => {
            trace!(target: "serial", request_count = reqs.len(), "enqueue: SendBack → work_queue");
            diag!("DIAG enqueue: SendBack({}) → work_queue", reqs.len());
            for req in reqs {
                work_queue.push_back(req);
            }
        }
        ActiveSessionOutput::SendAndThenReceive {
            send_request,
            then_receive_streams,
        } => {
            trace!(target: "serial", "enqueue: SendAndThenReceive → work_queue + deferred_streams");
            diag!(
                "DIAG enqueue: SendAndThenReceive → work_queue + {} deferred streams",
                then_receive_streams.len()
            );
            work_queue.push_back(send_request);
            merge_deferred_streams(deferred_streams, then_receive_streams);
        }
        ActiveSessionOutput::PendingReceive { desired_streams } => {
            trace!(target: "serial", streams = ?desired_streams, "enqueue: PendingReceive → deferred_streams");
            diag!(
                "DIAG enqueue: PendingReceive({}) → deferred_streams",
                desired_streams.len()
            );
            merge_deferred_streams(deferred_streams, desired_streams);
        }
        ActiveSessionOutput::HostCall(hc) => {
            diag!(
                "DIAG enqueue: HostCall method={} call_id={}",
                hc.method_name(),
                hc.call_id()
            );
            info!(target: "serial", method = %hc.method_name(), call_id = hc.call_id(), "enqueue: HostCall → pending_host_calls");
            pending_host_calls.push_back(hc);
        }
        ActiveSessionOutput::UserEvent(event) => {
            diag!("DIAG enqueue: UserEvent sending...");
            trace!(target: "serial", event = ?event, "enqueue: UserEvent → user_output_tx");
            if user_output_tx.send(event).await.is_err() {
                return Err(anyhow::anyhow!("User output channel disconnected"));
            }
            diag!("DIAG enqueue: UserEvent sent OK");
        }
        ActiveSessionOutput::SendBackError(e) => {
            error!(target: "serial", error = %e, "enqueue: SendBackError");
            return Err(anyhow::anyhow!("Session step failed: {e}"));
        }
        ActiveSessionOutput::OperationSuccess => {
            trace!(target: "serial", "enqueue: OperationSuccess (no-op)");
        }
        ActiveSessionOutput::Ignore => {
            // No-op
        }
    }
    Ok(())
}

/// Merge new desired streams into the deferred list, avoiding duplicates.
fn merge_deferred_streams(existing: &mut Vec<DesiredStream>, new_streams: Vec<DesiredStream>) {
    let before = existing.len();
    for s in new_streams {
        if !existing.contains(&s) {
            existing.push(s);
        }
    }
    if existing.len() != before {
        diag!(
            "DIAG merge_deferred: {} → {} streams",
            before,
            existing.len()
        );
    }
}

/// Dispatch the next queued HostCall to the consumer, if none is currently active.
fn try_dispatch_next_host_call(
    pending_host_calls: &mut VecDeque<HostCall>,
    host_call_active: &mut bool,
    host_call_tx: &mpsc::UnboundedSender<HostCall>,
) -> anyhow::Result<()> {
    if *host_call_active {
        return Ok(());
    }
    if let Some(hc) = pending_host_calls.pop_front() {
        diag!(
            "DIAG dispatch: HostCall method={} call_id={} ({} remaining)",
            hc.method_name(),
            hc.call_id(),
            pending_host_calls.len()
        );
        info!(
            target: "serial",
            method = %hc.method_name(),
            call_id = hc.call_id(),
            remaining = pending_host_calls.len(),
            "dispatching HostCall to consumer"
        );
        if host_call_tx.unbounded_send(hc).is_err() {
            return Err(anyhow::anyhow!("Host-call channel closed"));
        }
        *host_call_active = true;
    }
    Ok(())
}

/// Helper: get a short name for an `ActiveSessionOutput` variant (for diagnostics).
fn output_type_name(o: &ActiveSessionOutput) -> &'static str {
    match o {
        ActiveSessionOutput::SendBack(_) => "SendBack",
        ActiveSessionOutput::SendAndThenReceive { .. } => "SendAndThenReceive",
        ActiveSessionOutput::UserEvent(_) => "UserEvent",
        ActiveSessionOutput::HostCall(_) => "HostCall",
        ActiveSessionOutput::PendingReceive { .. } => "PendingReceive",
        ActiveSessionOutput::OperationSuccess => "OperationSuccess",
        ActiveSessionOutput::Ignore => "Ignore",
        ActiveSessionOutput::SendBackError(_) => "SendBackError",
    }
}
