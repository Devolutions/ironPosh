//! Protocol decision core for the serial session loop.
//!
//! Pure synchronous state machine. All queue management, promotion logic, and
//! protocol decisions live here. **No async, no channels** — fully unit-testable.
//!
//! The event loop ([`super::start_serial_session_loop`]) is a thin async shell
//! that shuttles data between I/O channels and this core.

use std::collections::VecDeque;

use anyhow::Context;
use ironposh_client_core::connector::active_session::{ActiveSession, UserEvent};
use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation, conntion_pool::TrySend};
use ironposh_client_core::host::HostCall;
use ironposh_client_core::runspace_pool::DesiredStream;
use tracing::{debug, error, info, trace};

use super::diag;
use crate::HostResponse;

/// Protocol decision core for the serial session loop.
///
/// Owns all internal queues and the [`ActiveSession`] state machine. Every
/// method is synchronous — the async event loop only needs to call these
/// methods and dispatch the resulting effects.
pub(super) struct SessionCore {
    active_session: ActiveSession,

    /// Pending HTTP requests (Command, HostResponse, Signal, etc.).
    /// **Always promoted before Receives.**
    work_queue: VecDeque<TrySend>,

    /// Demanded Receive streams — from `SendAndThenReceive`. The server is
    /// expected to have data ready after processing our preceding Send (e.g.,
    /// PSRP key exchange `EncryptedSessionKey`, host-response acknowledgement).
    /// **Always promoted** regardless of pipeline vs runspace-pool.
    demanded_streams: VecDeque<DesiredStream>,

    /// Speculative Receive streams — from `PendingReceive`. Pipeline streams
    /// are promoted; runspace-pool-only streams are **skipped** (would block for
    /// `OperationTimeout` with no useful data). Pipeline Receives already
    /// include runspace pool data, so nothing is lost.
    speculative_streams: Vec<DesiredStream>,

    /// HostCalls waiting to be dispatched to the JS/consumer side.
    pending_host_calls: VecDeque<HostCall>,

    /// Whether we're currently waiting for a host-call response from JS.
    host_call_active: bool,

    /// User operations buffered while an HTTP request is in flight.
    pending_user_ops: VecDeque<UserOperation>,

    /// User events accumulated during processing, to be drained by the event loop.
    pending_user_events: Vec<UserEvent>,
}

impl SessionCore {
    pub(super) fn new(first_receive: TrySend, active_session: ActiveSession) -> Self {
        let mut work_queue = VecDeque::new();
        work_queue.push_back(first_receive);
        Self {
            active_session,
            work_queue,
            demanded_streams: VecDeque::new(),
            speculative_streams: Vec::new(),
            pending_host_calls: VecDeque::new(),
            host_call_active: false,
            pending_user_ops: VecDeque::new(),
            pending_user_events: Vec::new(),
        }
    }

    // ── Buffered user ops ─────────────────────────────────────────────────

    /// Process ONE buffered user operation, if the connection is idle
    /// (`work_queue` is empty). Only one per call because
    /// `accept_client_operation` may itself call `send()`, moving the
    /// connection back to Pending.
    pub(super) fn process_one_buffered_op(&mut self) -> anyhow::Result<()> {
        if self.work_queue.is_empty()
            && let Some(op) = self.pending_user_ops.pop_front()
        {
            diag!("DIAG process buffered: {}", op.operation_type());
            debug!(target: "serial", operation = op.operation_type(), "processing buffered user operation");
            let output = self
                .active_session
                .accept_client_operation(op)
                .context("Failed to accept buffered user operation")?;
            self.route_output(output)?;
        }
        Ok(())
    }

    // ── Promotion ────────────────────────────────────────────────────────

    /// Pick the next HTTP request to send. Returns `None` when idle.
    ///
    /// Promotion priority:
    ///   1. `work_queue` (Send operations)
    ///   2. `demanded_streams` (server will respond — key exchange, etc.)
    ///   3. pipeline streams from `speculative_streams`
    ///   4. runspace-pool-only speculative → skip (returns `None`)
    pub(super) fn promote_next_request(&mut self) -> anyhow::Result<Option<TrySend>> {
        if let Some(req) = self.work_queue.pop_front() {
            diag!(
                "DIAG promote: sending from work_queue ({} remaining)",
                self.work_queue.len()
            );
            trace!(target: "serial", remaining_work = self.work_queue.len(), "promoting work_queue item");
            return Ok(Some(req));
        }

        // Only build Receives when all HostCalls are resolved. The server blocks
        // Receives while waiting for our HostCall response, so sending one early
        // would always time out (adding an unnecessary OperationTimeout delay).
        if self.host_call_active || !self.pending_host_calls.is_empty() {
            return Ok(None);
        }

        // Priority 1: demanded streams (from SendAndThenReceive).
        if let Some(stream) = self.demanded_streams.pop_front() {
            diag!(
                "DIAG promote: demanded Receive ({} demanded remaining, {} speculative)",
                self.demanded_streams.len(),
                self.speculative_streams.len()
            );
            trace!(
                target: "serial",
                ?stream,
                demanded_remaining = self.demanded_streams.len(),
                speculative_remaining = self.speculative_streams.len(),
                "promoting demanded stream to Receive"
            );
            let receive = self
                .active_session
                .fire_receive(vec![stream])
                .context("Failed to build Receive from demanded stream")?;
            return Ok(Some(receive));
        }

        // Priority 2: pipeline streams from speculative.
        let has_pipeline = self
            .speculative_streams
            .iter()
            .any(|s| s.command_id().is_some());
        if has_pipeline {
            self.speculative_streams
                .retain(|s| s.command_id().is_some());
            let stream = self.speculative_streams.remove(0);

            diag!(
                "DIAG promote: Receive for pipeline stream ({} speculative remaining)",
                self.speculative_streams.len()
            );
            trace!(
                target: "serial",
                ?stream,
                speculative_remaining = self.speculative_streams.len(),
                "promoting pipeline stream to Receive"
            );
            let receive = self
                .active_session
                .fire_receive(vec![stream])
                .context("Failed to build Receive from speculative stream")?;
            return Ok(Some(receive));
        }

        // Only speculative runspace-pool streams — skip to avoid blocking
        // for OperationTimeout with no useful data.
        if !self.speculative_streams.is_empty() {
            diag!(
                "DIAG promote: skipping speculative runspace-pool-only Receive (would block for OperationTimeout)"
            );
            trace!(target: "serial", "skipping speculative runspace-pool-only Receive, falling through to idle");
        }

        Ok(None)
    }

    // ── Server response ──────────────────────────────────────────────────

    /// Process an HTTP response from the server.
    pub(super) fn accept_response(
        &mut self,
        resp: crate::HttpResponseTargeted,
    ) -> anyhow::Result<()> {
        let outputs = match self.active_session.accept_server_response(resp) {
            Ok(outputs) => outputs,
            Err(e) => {
                diag!("DIAG ERROR: accept_server_response failed: {:#}", e);
                return Err(e).context("Failed to accept server response");
            }
        };

        let output_types: Vec<&str> = outputs.iter().map(output_type_name).collect();
        diag!("DIAG select: {} outputs: {:?}", outputs.len(), output_types);
        trace!(
            target: "serial",
            output_count = outputs.len(),
            ?output_types,
            "processing server response outputs"
        );

        for output in outputs {
            self.route_output(output)?;
        }
        Ok(())
    }

    // ── Idle processing (connection idle — safe to call accept_client_operation)

    /// Process a user operation when the connection is idle.
    pub(super) fn accept_user_op(&mut self, op: UserOperation) -> anyhow::Result<()> {
        diag!("DIAG idle: user op {}", op.operation_type());
        debug!(
            target: "serial",
            operation = op.operation_type(),
            "user operation received while idle"
        );
        let output = self
            .active_session
            .accept_client_operation(op)
            .context("Failed to accept user operation (idle)")?;
        self.route_output(output)
    }

    /// Process a host-call response when the connection is idle.
    pub(super) fn accept_host_response(&mut self, hr: HostResponse) -> anyhow::Result<()> {
        diag!("DIAG idle: host response received call_id={}", hr.call_id);
        debug!(
            target: "serial",
            call_id = hr.call_id,
            "host-call response received while idle"
        );
        self.host_call_active = false;
        let output = self
            .active_session
            .accept_client_operation(UserOperation::SubmitHostResponse {
                call_id: hr.call_id,
                scope: hr.scope,
                submission: hr.submission,
            })
            .context("Failed to submit host response (idle)")?;
        self.route_output(output)
    }

    // ── Buffering (HTTP in-flight — cannot call accept_client_operation) ─

    /// Buffer a user operation during HTTP in-flight.
    pub(super) fn buffer_user_op(&mut self, op: UserOperation) {
        diag!(
            "DIAG select: buffering user op {} (HTTP in flight)",
            op.operation_type()
        );
        debug!(
            target: "serial",
            operation = op.operation_type(),
            "buffering user operation (HTTP in flight)"
        );
        self.pending_user_ops.push_back(op);
    }

    /// Buffer a host-call response during HTTP in-flight.
    ///
    /// Converts to [`UserOperation::SubmitHostResponse`] for later processing,
    /// and marks `host_call_active = false` so the next HostCall can be dispatched.
    pub(super) fn buffer_host_response(&mut self, hr: HostResponse) {
        diag!(
            "DIAG select: buffering host response call_id={} (HTTP in flight)",
            hr.call_id
        );
        debug!(
            target: "serial",
            call_id = hr.call_id,
            "buffering host-call response (HTTP in flight)"
        );
        self.pending_user_ops
            .push_back(UserOperation::SubmitHostResponse {
                call_id: hr.call_id,
                scope: hr.scope,
                submission: hr.submission,
            });
        self.host_call_active = false;
    }

    // ── Effect draining ──────────────────────────────────────────────────

    /// Pop the next HostCall to dispatch, if none is currently active.
    pub(super) fn poll_host_call(&mut self) -> Option<HostCall> {
        if self.host_call_active {
            return None;
        }
        let hc = self.pending_host_calls.pop_front()?;
        diag!(
            "DIAG dispatch: HostCall method={} call_id={} ({} remaining)",
            hc.method_name(),
            hc.call_id(),
            self.pending_host_calls.len()
        );
        info!(
            target: "serial",
            method = %hc.method_name(),
            call_id = hc.call_id(),
            remaining = self.pending_host_calls.len(),
            "dispatching HostCall to consumer"
        );
        self.host_call_active = true;
        Some(hc)
    }

    /// Drain accumulated user events.
    pub(super) fn drain_user_events(&mut self) -> Vec<UserEvent> {
        std::mem::take(&mut self.pending_user_events)
    }

    /// Whether a HostCall is currently active (event loop uses this for `select!` guard).
    pub(super) fn is_host_call_active(&self) -> bool {
        self.host_call_active
    }

    // ── Internal routing ─────────────────────────────────────────────────

    /// Route an [`ActiveSessionOutput`] to the appropriate internal queue.
    /// **Never sends HTTP.** Never touches channels.
    fn route_output(&mut self, output: ActiveSessionOutput) -> anyhow::Result<()> {
        match output {
            ActiveSessionOutput::SendBack(reqs) => {
                trace!(target: "serial", request_count = reqs.len(), "enqueue: SendBack → work_queue");
                diag!("DIAG enqueue: SendBack({}) → work_queue", reqs.len());
                for req in reqs {
                    self.work_queue.push_back(req);
                }
            }
            ActiveSessionOutput::SendAndThenReceive {
                send_request,
                then_receive_streams,
            } => {
                // "I just sent something, the server WILL respond."
                // Follow-up streams go to demanded_streams (always promoted).
                trace!(
                    target: "serial",
                    stream_count = then_receive_streams.len(),
                    "enqueue: SendAndThenReceive → work_queue + demanded_streams"
                );
                diag!(
                    "DIAG enqueue: SendAndThenReceive → work_queue + {} demanded streams",
                    then_receive_streams.len()
                );
                self.work_queue.push_back(send_request);
                for s in then_receive_streams {
                    self.demanded_streams.push_back(s);
                }
            }
            ActiveSessionOutput::PendingReceive { desired_streams } => {
                // Speculative: "you should eventually poll these."
                trace!(target: "serial", streams = ?desired_streams, "enqueue: PendingReceive → speculative_streams");
                diag!(
                    "DIAG enqueue: PendingReceive({}) → speculative_streams",
                    desired_streams.len()
                );
                merge_speculative_streams(&mut self.speculative_streams, desired_streams);
            }
            ActiveSessionOutput::HostCall(hc) => {
                diag!(
                    "DIAG enqueue: HostCall method={} call_id={}",
                    hc.method_name(),
                    hc.call_id()
                );
                info!(target: "serial", method = %hc.method_name(), call_id = hc.call_id(), "enqueue: HostCall → pending_host_calls");
                self.pending_host_calls.push_back(hc);
            }
            ActiveSessionOutput::UserEvent(event) => {
                diag!("DIAG enqueue: UserEvent queued");
                trace!(target: "serial", event = ?event, "enqueue: UserEvent → pending_user_events");
                self.pending_user_events.push(event);
            }
            ActiveSessionOutput::SendBackError(e) => {
                error!(target: "serial", error = %e, "enqueue: SendBackError");
                return Err(anyhow::anyhow!("Session step failed: {e}"));
            }
            ActiveSessionOutput::OperationSuccess => {
                trace!(target: "serial", "enqueue: OperationSuccess (no-op)");
            }
            ActiveSessionOutput::Ignore => {}
        }
        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Merge new desired streams into the speculative list, avoiding duplicates.
fn merge_speculative_streams(existing: &mut Vec<DesiredStream>, new_streams: Vec<DesiredStream>) {
    let before = existing.len();
    for s in new_streams {
        if !existing.contains(&s) {
            existing.push(s);
        }
    }
    if existing.len() != before {
        diag!(
            "DIAG merge_speculative: {} → {} streams",
            before,
            existing.len()
        );
    }
}

/// Short name for an [`ActiveSessionOutput`] variant (for diagnostics).
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
