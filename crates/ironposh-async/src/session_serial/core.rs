//! Protocol decision core for the serial session loop.
//!
//! Pure synchronous state machine. All queue management, promotion logic, and
//! protocol decisions live here. **No async, no channels** — fully unit-testable.
//!
//! The event loop ([`super::start_serial_session_loop`]) is a thin async shell
//! that shuttles data between I/O channels and this core.

use std::collections::VecDeque;

use anyhow::Context;
use ironposh_client_core::PwshCoreError;
use ironposh_client_core::connector::active_session::{ActiveSession, UserEvent};
use ironposh_client_core::connector::http::HttpResponseTargeted;
use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation, conntion_pool::TrySend};
use ironposh_client_core::host::HostCall;
use ironposh_client_core::runspace_pool::DesiredStream;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

use super::diag;
use super::scheduler::{DefaultReceiveScheduler, ReceiveScheduler, TargetId};
use crate::HostResponse;
use crate::clock::Instant;

// ── Backend trait ─────────────────────────────────────────────────────────

/// Abstraction over [`ActiveSession`] so that [`SessionCore`] can be tested
/// with a mock backend that doesn't require a real WinRM connection.
pub(super) trait SessionBackend {
    fn accept_client_operation(
        &mut self,
        op: UserOperation,
    ) -> Result<ActiveSessionOutput, PwshCoreError>;

    fn accept_server_response(
        &mut self,
        resp: HttpResponseTargeted,
    ) -> Result<Vec<ActiveSessionOutput>, PwshCoreError>;

    fn fire_receive(&mut self, streams: Vec<DesiredStream>) -> Result<TrySend, PwshCoreError>;
}

impl SessionBackend for ActiveSession {
    fn accept_client_operation(
        &mut self,
        op: UserOperation,
    ) -> Result<ActiveSessionOutput, PwshCoreError> {
        Self::accept_client_operation(self, op)
    }

    fn accept_server_response(
        &mut self,
        resp: HttpResponseTargeted,
    ) -> Result<Vec<ActiveSessionOutput>, PwshCoreError> {
        Self::accept_server_response(self, resp)
    }

    fn fire_receive(&mut self, streams: Vec<DesiredStream>) -> Result<TrySend, PwshCoreError> {
        Self::fire_receive(self, streams)
    }
}

// ── Send priority ─────────────────────────────────────────────────────────

/// Whether a `SendBack` should be pushed to the front or back of the work queue.
///
/// `Front` is used for user-initiated Signals (Ctrl+C / KillPipeline) so they
/// get maximum priority in the single-connection serial loop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SendPriority {
    Normal,
    Front,
}

impl SendPriority {
    /// Compute the priority for a user operation — `Front` for KillPipeline,
    /// `Normal` for everything else.
    fn for_user_op(op: &UserOperation) -> Self {
        if matches!(op, UserOperation::KillPipeline { .. }) {
            Self::Front
        } else {
            Self::Normal
        }
    }
}

/// Protocol decision core for the serial session loop.
///
/// Owns all internal queues and the [`ActiveSession`] state machine. Every
/// method is synchronous — the async event loop only needs to call these
/// methods and dispatch the resulting effects.
pub(super) struct SessionCore<S: SessionBackend = ActiveSession> {
    active_session: S,
    epoch: Instant,
    scheduler: DefaultReceiveScheduler,
    next_wakeup_at_ms: Option<u64>,
    in_flight_receive_target: Option<TargetId>,

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
        Self::new_with_backend(first_receive, active_session)
    }
}

impl<S: SessionBackend> SessionCore<S> {
    fn new_with_backend(first_receive: TrySend, active_session: S) -> Self {
        let mut work_queue = VecDeque::new();
        work_queue.push_back(first_receive);
        Self {
            active_session,
            epoch: Instant::now(),
            scheduler: DefaultReceiveScheduler::new(),
            next_wakeup_at_ms: None,
            in_flight_receive_target: None,
            work_queue,
            demanded_streams: VecDeque::new(),
            speculative_streams: Vec::new(),
            pending_host_calls: VecDeque::new(),
            host_call_active: false,
            pending_user_ops: VecDeque::new(),
            pending_user_events: Vec::new(),
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    pub(super) fn next_wakeup_in_ms(&self, now_ms: u64) -> Option<u64> {
        let at = self.next_wakeup_at_ms?;
        Some(at.saturating_sub(now_ms))
    }

    // ── Buffered user ops ─────────────────────────────────────────────────

    /// Process ONE buffered user operation, if the connection is idle
    /// (`work_queue` is empty). Only one per call because
    /// `accept_client_operation` may itself call `send()`, moving the
    /// connection back to Pending.
    pub(super) fn process_one_buffered_op(&mut self) -> anyhow::Result<()> {
        let can_process = if self.work_queue.is_empty() {
            true
        } else {
            matches!(
                self.pending_user_ops.front(),
                Some(UserOperation::KillPipeline { .. })
            )
        };

        if can_process && let Some(op) = self.pending_user_ops.pop_front() {
            diag!("DIAG process buffered: {}", op.operation_type());
            debug!(target: "serial", operation = op.operation_type(), "processing buffered user operation");
            let priority = SendPriority::for_user_op(&op);
            self.observe_user_op(&op);
            let output = self
                .active_session
                .accept_client_operation(op)
                .context("Failed to accept buffered user operation")?;
            self.route_output(output, priority)?;
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
        self.next_wakeup_at_ms = None;
        let now_ms = self.now_ms();

        if let Some(req) = self.try_promote_work_queue() {
            return Ok(Some(req));
        }

        // Only build Receives when all HostCalls are resolved. The server blocks
        // Receives while waiting for our HostCall response, so sending one early
        // would always time out (adding an unnecessary OperationTimeout delay).
        if self.host_call_active || !self.pending_host_calls.is_empty() {
            return Ok(None);
        }

        if let Some(receive) = self.try_promote_demanded_receive(now_ms)? {
            return Ok(Some(receive));
        }

        self.try_promote_speculative_receive(now_ms)
    }

    /// Pop the next item from the work queue (Send operations).
    fn try_promote_work_queue(&mut self) -> Option<TrySend> {
        let req = self.work_queue.pop_front()?;
        diag!(
            "DIAG promote: sending from work_queue ({} remaining)",
            self.work_queue.len()
        );
        trace!(target: "serial", remaining_work = self.work_queue.len(), "promoting work_queue item");
        self.in_flight_receive_target = None;
        Some(req)
    }

    /// Try to promote a demanded Receive (from `SendAndThenReceive`).
    ///
    /// Skips streams that are scheduler-blocked and records wakeup times.
    fn try_promote_demanded_receive(&mut self, now_ms: u64) -> anyhow::Result<Option<TrySend>> {
        while let Some(stream) = self.demanded_streams.pop_front() {
            let target = TargetId::from_stream(&stream);
            if !self.scheduler.is_allowed_target(target, now_ms) {
                if let Some(at) = self.scheduler.next_eligible_at_ms(target) {
                    self.next_wakeup_at_ms =
                        Some(self.next_wakeup_at_ms.map_or(at, |cur| cur.min(at)));
                }
                // Cancelled targets (None) are silently dropped — no wakeup needed.
                continue;
            }

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
            self.in_flight_receive_target = Some(target);
            return Ok(Some(receive));
        }
        Ok(None)
    }

    /// Try to promote a speculative pipeline Receive.
    ///
    /// Skips runspace-pool-only streams (would block for OperationTimeout with
    /// no useful data). Pipeline Receives already include runspace pool data.
    fn try_promote_speculative_receive(&mut self, now_ms: u64) -> anyhow::Result<Option<TrySend>> {
        let mut earliest_blocked: Option<u64> = None;
        let mut idx_to_take: Option<usize> = None;
        for (idx, s) in self.speculative_streams.iter().enumerate() {
            if s.command_id().is_none() {
                continue;
            }
            let target = TargetId::from_stream(s);
            if self.scheduler.is_allowed_target(target, now_ms) {
                idx_to_take = Some(idx);
                break;
            }
            if let Some(at) = self.scheduler.next_eligible_at_ms(target) {
                earliest_blocked = Some(earliest_blocked.map_or(at, |cur| cur.min(at)));
            }
        }

        if let Some(idx) = idx_to_take {
            let stream = self.speculative_streams.remove(idx);
            let target = TargetId::from_stream(&stream);

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
            self.in_flight_receive_target = Some(target);
            return Ok(Some(receive));
        }

        if let Some(at) = earliest_blocked {
            self.next_wakeup_at_ms = Some(self.next_wakeup_at_ms.map_or(at, |cur| cur.min(at)));
            return Ok(None);
        }

        // Only speculative runspace-pool streams — skip to avoid blocking
        // for OperationTimeout with no useful data.
        if !self.speculative_streams.is_empty() {
            diag!(
                "DIAG promote: skipping speculative runspace-pool-only Receive (would block for OperationTimeout)"
            );
            trace!(target: "serial", "skipping speculative runspace-pool-only Receive, falling through to idle");
        }

        self.in_flight_receive_target = None;
        Ok(None)
    }

    // ── Server response ──────────────────────────────────────────────────

    /// Process an HTTP response from the server.
    pub(super) fn accept_response(&mut self, resp: HttpResponseTargeted) -> anyhow::Result<()> {
        let now_ms = self.now_ms();
        let in_flight_receive = self.in_flight_receive_target.take();

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

        if let Some(target) = in_flight_receive {
            let timeout_like = outputs.iter().all(|o| {
                matches!(
                    o,
                    ActiveSessionOutput::PendingReceive { .. }
                        | ActiveSessionOutput::OperationSuccess
                        | ActiveSessionOutput::Ignore
                )
            });

            if timeout_like {
                self.scheduler.note_receive_timeout(target, now_ms);
            } else {
                self.scheduler.note_receive_progress(target, now_ms);
            }
        }

        for output in outputs {
            self.route_output(output, SendPriority::Normal)?;
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
        let priority = SendPriority::for_user_op(&op);
        self.observe_user_op(&op);
        let output = self
            .active_session
            .accept_client_operation(op)
            .context("Failed to accept user operation (idle)")?;
        self.route_output(output, priority)
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
        self.route_output(output, SendPriority::Normal)
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
        self.observe_user_op(&op);
        if matches!(op, UserOperation::KillPipeline { .. }) {
            self.pending_user_ops.push_front(op);
        } else {
            self.pending_user_ops.push_back(op);
        }
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
    ///
    /// `priority` only affects `SendBack`: `Front` pushes to the head of the
    /// work queue (used for KillPipeline signals).
    fn route_output(
        &mut self,
        output: ActiveSessionOutput,
        priority: SendPriority,
    ) -> anyhow::Result<()> {
        match output {
            ActiveSessionOutput::SendBack(reqs) => {
                trace!(target: "serial", request_count = reqs.len(), "enqueue: SendBack → work_queue");
                diag!("DIAG enqueue: SendBack({}) → work_queue", reqs.len());
                if priority == SendPriority::Front {
                    // Preserve order while pushing to the front.
                    for req in reqs.into_iter().rev() {
                        self.work_queue.push_front(req);
                    }
                } else {
                    for req in reqs {
                        self.work_queue.push_back(req);
                    }
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
                if let UserEvent::PipelineFinished { pipeline } = &event {
                    self.scheduler
                        .note_pipeline_finished(pipeline.id(), self.now_ms());
                }
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

    fn observe_user_op(&mut self, op: &UserOperation) {
        let UserOperation::KillPipeline { pipeline } = op else {
            return;
        };

        let pipeline_id: Uuid = pipeline.id();
        let now_ms = self.now_ms();
        self.scheduler.note_cancel_requested(pipeline_id, now_ms);

        info!(
            target: "serial",
            pipeline_id = %pipeline_id,
            "scheduler: cancel requested"
        );
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

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_client_core::connector::conntion_pool::ConnectionId;
    use ironposh_client_core::connector::http::{HttpRequest, Method};
    use ironposh_client_core::host::{HostCallScope, Transport};
    use ironposh_client_core::powershell::PipelineHandle;
    use std::collections::VecDeque;

    // ── Helpers ──────────────────────────────────────────────────────────

    /// Minimal mock backend. Each `SessionBackend` method pops from the
    /// corresponding response queue. Panics if a queue is exhausted.
    struct MockBackend {
        op_responses: VecDeque<ActiveSessionOutput>,
        receive_results: VecDeque<TrySend>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                op_responses: VecDeque::new(),
                receive_results: VecDeque::new(),
            }
        }
    }

    impl SessionBackend for MockBackend {
        fn accept_client_operation(
            &mut self,
            _op: UserOperation,
        ) -> Result<ActiveSessionOutput, PwshCoreError> {
            Ok(self
                .op_responses
                .pop_front()
                .expect("MockBackend: op_responses exhausted"))
        }

        fn accept_server_response(
            &mut self,
            _resp: HttpResponseTargeted,
        ) -> Result<Vec<ActiveSessionOutput>, PwshCoreError> {
            unimplemented!("accept_server_response not needed for these tests")
        }

        fn fire_receive(&mut self, _streams: Vec<DesiredStream>) -> Result<TrySend, PwshCoreError> {
            Ok(self
                .receive_results
                .pop_front()
                .expect("MockBackend: receive_results exhausted"))
        }
    }

    /// Build a dummy `TrySend::JustSend`.
    fn dummy_try_send(conn_id: u32) -> TrySend {
        TrySend::JustSend {
            request: HttpRequest {
                method: Method::Post,
                url: "http://test/wsman".to_string(),
                headers: vec![],
                body: None,
                cookie: None,
            },
            conn_id: ConnectionId::test_new(conn_id),
        }
    }

    /// Create a `SessionCore<S>` with an empty work queue.
    fn core_idle<S: SessionBackend>(mock: S) -> SessionCore<S> {
        let mut core = SessionCore::new_with_backend(dummy_try_send(1), mock);
        // Drain the initial first_receive so the work queue starts empty.
        let _ = core.work_queue.pop_front();
        core
    }

    /// Create a pipeline `DesiredStream` (has a `command_id`).
    fn pipeline_stream(id: Uuid) -> DesiredStream {
        DesiredStream::test_new("stdout", Some(id))
    }

    /// Create a runspace-pool `DesiredStream` (no `command_id`).
    fn runspace_stream() -> DesiredStream {
        DesiredStream::test_new("stdout", None)
    }

    /// Build a minimal `HostCall` for testing (GetName, simplest variant).
    fn dummy_host_call(call_id: i64) -> HostCall {
        HostCall::GetName {
            transport: Transport::new(HostCallScope::RunspacePool, call_id, ()),
        }
    }

    /// Build a `PipelineHandle` for testing.
    fn pipeline_handle(id: Uuid) -> PipelineHandle {
        PipelineHandle::new(id)
    }

    // ── Promotion priority (6 tests) ────────────────────────────────────

    #[test]
    fn work_queue_items_promoted_before_receives() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(10));

        let mut core = SessionCore::new_with_backend(dummy_try_send(1), mock);
        // work_queue has the initial first_receive. Add a demanded stream too.
        let id = Uuid::new_v4();
        core.demanded_streams.push_back(pipeline_stream(id));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_some(), "should promote from work_queue");
        assert_eq!(promoted.unwrap().get_connection_id().inner(), 1);
    }

    #[test]
    fn demanded_streams_promoted_before_speculative() {
        let mut mock = MockBackend::new();
        // fire_receive will be called for the demanded stream.
        mock.receive_results.push_back(dummy_try_send(20));

        let mut core = core_idle(mock);
        let id = Uuid::new_v4();
        core.demanded_streams.push_back(pipeline_stream(id));
        core.speculative_streams
            .push(pipeline_stream(Uuid::new_v4()));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_some());
        assert_eq!(promoted.unwrap().get_connection_id().inner(), 20);
        // The speculative stream should still be there.
        assert_eq!(core.speculative_streams.len(), 1);
    }

    #[test]
    fn speculative_runspace_pool_only_returns_none() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        // Only runspace-pool streams (no command_id) → should skip.
        core.speculative_streams.push(runspace_stream());

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_none());
    }

    #[test]
    fn speculative_pipeline_stream_promoted() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(30));

        let mut core = core_idle(mock);
        let id = Uuid::new_v4();
        core.speculative_streams.push(pipeline_stream(id));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_some());
        assert_eq!(promoted.unwrap().get_connection_id().inner(), 30);
    }

    #[test]
    fn host_call_active_blocks_receive_promotion() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(40));

        let mut core = core_idle(mock);
        core.demanded_streams
            .push_back(pipeline_stream(Uuid::new_v4()));
        core.host_call_active = true;

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_none());
    }

    #[test]
    fn pending_host_calls_block_receive_promotion() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(50));

        let mut core = core_idle(mock);
        core.demanded_streams
            .push_back(pipeline_stream(Uuid::new_v4()));
        core.pending_host_calls.push_back(dummy_host_call(1));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_none());
    }

    // ── Routing (7 tests) ───────────────────────────────────────────────

    #[test]
    fn route_send_back_normal_appends() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        // Pre-populate work_queue so we can verify append.
        core.work_queue.push_back(dummy_try_send(1));

        let output = ActiveSessionOutput::SendBack(vec![dummy_try_send(2)]);
        core.route_output(output, SendPriority::Normal).unwrap();

        assert_eq!(core.work_queue.len(), 2);
        // First item should still be conn_id=1.
        assert_eq!(core.work_queue[0].get_connection_id().inner(), 1);
        assert_eq!(core.work_queue[1].get_connection_id().inner(), 2);
    }

    #[test]
    fn route_send_back_front_prepends() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        core.work_queue.push_back(dummy_try_send(1));

        let output = ActiveSessionOutput::SendBack(vec![dummy_try_send(2)]);
        core.route_output(output, SendPriority::Front).unwrap();

        assert_eq!(core.work_queue.len(), 2);
        // Front-priority item should be first.
        assert_eq!(core.work_queue[0].get_connection_id().inner(), 2);
        assert_eq!(core.work_queue[1].get_connection_id().inner(), 1);
    }

    #[test]
    fn route_send_and_then_receive_populates_both_queues() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        let id = Uuid::new_v4();

        let output = ActiveSessionOutput::SendAndThenReceive {
            send_request: dummy_try_send(3),
            then_receive_streams: vec![pipeline_stream(id)],
        };
        core.route_output(output, SendPriority::Normal).unwrap();

        assert_eq!(core.work_queue.len(), 1);
        assert_eq!(core.demanded_streams.len(), 1);
    }

    #[test]
    fn route_pending_receive_deduplicates() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        let id = Uuid::new_v4();
        let stream = pipeline_stream(id);

        // Add the same stream twice via PendingReceive.
        core.route_output(
            ActiveSessionOutput::PendingReceive {
                desired_streams: vec![stream.clone()],
            },
            SendPriority::Normal,
        )
        .unwrap();
        core.route_output(
            ActiveSessionOutput::PendingReceive {
                desired_streams: vec![stream],
            },
            SendPriority::Normal,
        )
        .unwrap();

        assert_eq!(
            core.speculative_streams.len(),
            1,
            "streams should be deduplicated"
        );
    }

    #[test]
    fn route_user_event_pipeline_finished_notifies_scheduler() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        let id = Uuid::new_v4();

        let event = UserEvent::PipelineFinished {
            pipeline: pipeline_handle(id),
        };
        core.route_output(ActiveSessionOutput::UserEvent(event), SendPriority::Normal)
            .unwrap();

        // After PipelineFinished, the scheduler should block the pipeline target.
        let target = TargetId::Pipeline(id);
        assert!(
            !core.scheduler.is_allowed_target(target, core.now_ms()),
            "scheduler should block finished pipeline"
        );
    }

    #[test]
    fn route_send_back_error_returns_err() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);

        let output = ActiveSessionOutput::SendBackError(PwshCoreError::InternalError(
            "test error".to_string(),
        ));
        let result = core.route_output(output, SendPriority::Normal);
        assert!(result.is_err());
    }

    #[test]
    fn route_host_call_queued() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);

        let hc = dummy_host_call(42);
        core.route_output(ActiveSessionOutput::HostCall(hc), SendPriority::Normal)
            .unwrap();

        assert_eq!(core.pending_host_calls.len(), 1);
        assert_eq!(core.pending_host_calls[0].call_id(), 42);
    }

    // ── Host-call state (3 tests) ───────────────────────────────────────

    #[test]
    fn poll_host_call_returns_none_when_active() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        core.pending_host_calls.push_back(dummy_host_call(1));
        core.host_call_active = true;

        assert!(core.poll_host_call().is_none());
    }

    #[test]
    fn poll_host_call_sets_active_flag() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        core.pending_host_calls.push_back(dummy_host_call(1));

        assert!(!core.host_call_active);
        let hc = core.poll_host_call();
        assert!(hc.is_some());
        assert!(core.host_call_active);
    }

    #[test]
    fn buffer_host_response_clears_active_flag() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        core.host_call_active = true;

        core.buffer_host_response(HostResponse {
            call_id: 1,
            scope: HostCallScope::RunspacePool,
            submission: ironposh_client_core::host::Submission::NoSend,
        });

        assert!(!core.host_call_active);
        // Should have buffered a SubmitHostResponse op.
        assert_eq!(core.pending_user_ops.len(), 1);
    }

    // ── Buffering & processing (4 tests) ────────────────────────────────

    #[test]
    fn buffer_kill_pipeline_goes_to_front() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // Buffer a normal op first.
        core.buffer_user_op(UserOperation::InvokeWithSpec {
            uuid: id1,
            spec: ironposh_client_core::pipeline::PipelineSpec {
                commands: vec![ironposh_client_core::pipeline::PipelineCommand::new_script(
                    "test".to_string(),
                )],
            },
        });

        // Buffer a KillPipeline — should go to front.
        core.buffer_user_op(UserOperation::KillPipeline {
            pipeline: pipeline_handle(id2),
        });

        assert!(matches!(
            core.pending_user_ops.front(),
            Some(UserOperation::KillPipeline { .. })
        ));
    }

    #[test]
    fn process_buffered_op_skips_when_work_queue_nonempty() {
        let mut mock = MockBackend::new();
        // This response should NOT be consumed since we skip processing.
        mock.op_responses
            .push_back(ActiveSessionOutput::OperationSuccess);

        let mut core = SessionCore::new_with_backend(dummy_try_send(1), mock);
        // work_queue has the initial item. Buffer a non-kill op.
        core.pending_user_ops
            .push_back(UserOperation::InvokeWithSpec {
                uuid: Uuid::new_v4(),
                spec: ironposh_client_core::pipeline::PipelineSpec {
                    commands: vec![ironposh_client_core::pipeline::PipelineCommand::new_script(
                        "test".to_string(),
                    )],
                },
            });

        core.process_one_buffered_op().unwrap();
        // The op should still be buffered because work_queue was non-empty.
        assert_eq!(core.pending_user_ops.len(), 1);
    }

    #[test]
    fn process_buffered_op_allows_kill_even_with_work_queue() {
        let mut mock = MockBackend::new();
        // KillPipeline produces SendBack.
        mock.op_responses
            .push_back(ActiveSessionOutput::SendBack(vec![dummy_try_send(99)]));

        let mut core = SessionCore::new_with_backend(dummy_try_send(1), mock);
        let id = Uuid::new_v4();
        core.pending_user_ops
            .push_back(UserOperation::KillPipeline {
                pipeline: pipeline_handle(id),
            });

        core.process_one_buffered_op().unwrap();
        // The KillPipeline should have been processed even with work_queue non-empty.
        assert_eq!(core.pending_user_ops.len(), 0);
        // And the SendBack from KillPipeline should be at the FRONT (priority=Front).
        assert_eq!(core.work_queue[0].get_connection_id().inner(), 99);
    }

    #[test]
    fn drain_user_events_empties_buffer() {
        let mock = MockBackend::new();
        let mut core = core_idle(mock);
        let id = Uuid::new_v4();

        core.pending_user_events.push(UserEvent::PipelineCreated {
            pipeline: pipeline_handle(id),
        });

        let events = core.drain_user_events();
        assert_eq!(events.len(), 1);

        let events2 = core.drain_user_events();
        assert!(events2.is_empty());
    }

    // ── Scheduler integration (2 tests) ─────────────────────────────────

    #[test]
    fn scheduler_cancelled_demanded_does_not_set_wakeup() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(60));

        let mut core = core_idle(mock);
        let id = Uuid::new_v4();
        // Cancel the pipeline target — should be permanently dropped, no wakeup.
        core.scheduler.note_cancel_requested(id, 0);
        core.demanded_streams.push_back(pipeline_stream(id));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_none());
        // Cancelled targets should NOT set a wakeup (they are permanently dropped).
        assert!(
            core.next_wakeup_at_ms.is_none(),
            "cancelled stream should not schedule a wakeup"
        );
    }

    #[test]
    fn scheduler_backed_off_demanded_sets_wakeup() {
        let mut mock = MockBackend::new();
        mock.receive_results.push_back(dummy_try_send(60));

        let mut core = core_idle(mock);
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);
        // Trigger backoff (not cancellation) — should set a wakeup.
        core.scheduler.note_receive_timeout(target, 0);
        core.demanded_streams.push_back(pipeline_stream(id));

        let promoted = core.promote_next_request().unwrap();
        assert!(promoted.is_none());
        // A backed-off (not cancelled) target should set a wakeup.
        assert!(
            core.next_wakeup_at_ms.is_some(),
            "backed-off stream should schedule a wakeup"
        );
    }

    #[test]
    fn accept_response_timeout_triggers_backoff() {
        // This test verifies that timeout-like responses trigger scheduler backoff.
        // We need accept_server_response, so we use a specialized mock.
        struct TimeoutMock;
        impl SessionBackend for TimeoutMock {
            fn accept_client_operation(
                &mut self,
                _op: UserOperation,
            ) -> Result<ActiveSessionOutput, PwshCoreError> {
                unimplemented!()
            }
            fn accept_server_response(
                &mut self,
                _resp: HttpResponseTargeted,
            ) -> Result<Vec<ActiveSessionOutput>, PwshCoreError> {
                // Return timeout-like outputs.
                Ok(vec![ActiveSessionOutput::PendingReceive {
                    desired_streams: vec![DesiredStream::test_new("stdout", Some(Uuid::new_v4()))],
                }])
            }
            fn fire_receive(
                &mut self,
                _streams: Vec<DesiredStream>,
            ) -> Result<TrySend, PwshCoreError> {
                Ok(dummy_try_send(70))
            }
        }

        let mut core = core_idle(TimeoutMock);
        let pipeline_id = Uuid::new_v4();
        let target = TargetId::Pipeline(pipeline_id);

        // Simulate an in-flight Receive for this target.
        core.in_flight_receive_target = Some(target);

        // Build a minimal HttpResponseTargeted to pass to accept_response.
        // We need ConnectionId and HttpResponse. Using test_new for ConnectionId.
        let resp = HttpResponseTargeted::new(
            ironposh_client_core::connector::http::HttpResponse {
                status_code: 200,
                headers: vec![],
                body: ironposh_client_core::connector::http::HttpBody::Xml(String::new()),
            },
            ConnectionId::test_new(1),
            None,
        );
        core.accept_response(resp).unwrap();

        // After a timeout-like response, the scheduler should have applied backoff.
        assert!(
            !core.scheduler.is_allowed_target(target, core.now_ms()),
            "scheduler should have applied backoff after timeout"
        );
    }
}
