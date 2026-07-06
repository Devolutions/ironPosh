use std::collections::HashMap;

use ironposh_client_core::runspace_pool::DesiredStream;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetId {
    RunspacePool,
    Pipeline(Uuid),
}

impl TargetId {
    pub fn from_stream(s: &DesiredStream) -> Self {
        s.command_id()
            .map_or(Self::RunspacePool, |id| Self::Pipeline(*id))
    }
}

#[derive(Debug, Default, Clone)]
struct TargetState {
    timeout_streak: u32,
    finished: bool,
    cancel_requested_at_ms: Option<u64>,
}

pub trait ReceiveScheduler {
    fn note_cancel_requested(&mut self, pipeline_id: Uuid, now_ms: u64);
    fn note_pipeline_finished(&mut self, pipeline_id: Uuid, now_ms: u64);
    fn note_receive_timeout(&mut self, target: TargetId, now_ms: u64);
    fn note_receive_progress(&mut self, target: TargetId, now_ms: u64);
    fn note_user_activity(&mut self, now_ms: u64);

    fn is_allowed_target(&self, target: TargetId, now_ms: u64) -> bool;
    /// Returns the earliest time this target may be polled, or `None` if the
    /// target is finished and should never be scheduled again.
    fn next_eligible_at_ms(&self, target: TargetId) -> Option<u64>;

    /// Server-side hold (Receive OperationTimeout) in milliseconds for the next
    /// poll of this target.
    fn receive_hold_ms(&self, target: TargetId, now_ms: u64) -> u64;
}

/// Default serial scheduler policy:
/// - if a pipeline is finished, never poll it again
/// - after cancel is requested, cap the hold so we observe the finish quickly
/// - grow the server-side hold on repeated empty polls so the connection always
///   has a Receive parked instead of idling on a client-side backoff sleep
#[derive(Debug, Default)]
pub struct DefaultReceiveScheduler {
    targets: HashMap<TargetId, TargetState>,
    base_hold_ms: u64,
    max_hold_ms: u64,
    max_hold_after_cancel_ms: u64,
    activity_cap_ms: u64,
    activity_window_ms: u64,
    last_user_activity_ms: Option<u64>,
}

impl DefaultReceiveScheduler {
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            base_hold_ms: 250,
            max_hold_ms: 1_000,
            max_hold_after_cancel_ms: 500,
            activity_cap_ms: 250,
            activity_window_ms: 3_000,
            last_user_activity_ms: None,
        }
    }

    fn state_mut(&mut self, id: TargetId) -> &mut TargetState {
        self.targets.entry(id).or_default()
    }

    fn state(&self, id: TargetId) -> Option<&TargetState> {
        self.targets.get(&id)
    }

    fn recently_active(&self, now_ms: u64) -> bool {
        self.last_user_activity_ms
            .is_some_and(|last| now_ms.saturating_sub(last) <= self.activity_window_ms)
    }
}

impl ReceiveScheduler for DefaultReceiveScheduler {
    fn note_cancel_requested(&mut self, pipeline_id: Uuid, now_ms: u64) {
        let st = self.state_mut(TargetId::Pipeline(pipeline_id));
        // Cancellation is cooperative. Keep polling this target (short holds)
        // so we can observe PipelineFinished or a non-fatal InvalidSelectors
        // fault that will be translated into PipelineFinished by the backend.
        st.cancel_requested_at_ms = Some(now_ms);
        st.timeout_streak = 0;
    }

    fn note_pipeline_finished(&mut self, pipeline_id: Uuid, _now_ms: u64) {
        let st = self.state_mut(TargetId::Pipeline(pipeline_id));
        st.finished = true;
    }

    fn note_receive_timeout(&mut self, target: TargetId, _now_ms: u64) {
        // No client-side sleep: an empty poll just grows the next server-side
        // hold via the streak. The connection stays parked on a Receive.
        let st = self.state_mut(target);
        st.timeout_streak = st.timeout_streak.saturating_add(1);
    }

    fn note_receive_progress(&mut self, target: TargetId, _now_ms: u64) {
        let st = self.state_mut(target);
        st.timeout_streak = 0;
    }

    fn note_user_activity(&mut self, now_ms: u64) {
        self.last_user_activity_ms = Some(now_ms);
    }

    fn is_allowed_target(&self, target: TargetId, _now_ms: u64) -> bool {
        self.state(target).is_none_or(|st| !st.finished)
    }

    fn next_eligible_at_ms(&self, target: TargetId) -> Option<u64> {
        match self.state(target) {
            Some(st) if st.finished => None,
            _ => Some(0),
        }
    }

    fn receive_hold_ms(&self, target: TargetId, now_ms: u64) -> u64 {
        let st = self.state(target);
        let streak = st.map_or(0, |s| s.timeout_streak);
        let exp = streak.min(31);
        let pow = 1u64 << exp;
        let mut hold = self.base_hold_ms.saturating_mul(pow).min(self.max_hold_ms);

        if st.is_some_and(|s| s.cancel_requested_at_ms.is_some()) {
            hold = hold.min(self.max_hold_after_cancel_ms);
        }
        if self.recently_active(now_ms) {
            hold = hold.min(self.activity_cap_ms);
        }
        hold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_requested_does_not_permanently_block_pipeline_stream() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        assert!(sched.is_allowed_target(target, 0));
        sched.note_cancel_requested(id, 1000);
        assert!(sched.is_allowed_target(target, 1000));
    }

    #[test]
    fn hold_doubles_per_empty_poll_and_caps_at_one_second() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        // Fresh target: base hold.
        assert_eq!(sched.receive_hold_ms(target, 0), 250);
        sched.note_receive_timeout(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 0), 500);
        sched.note_receive_timeout(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 0), 1_000);
        // Further empty polls stay capped at 1s.
        sched.note_receive_timeout(target, 0);
        sched.note_receive_timeout(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 0), 1_000);
    }

    #[test]
    fn progress_resets_hold_to_base() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        sched.note_receive_timeout(target, 0);
        sched.note_receive_timeout(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 0), 1_000);
        sched.note_receive_progress(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 0), 250);
    }

    #[test]
    fn recent_activity_caps_hold_at_quarter_second() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        // Grow the streak so the uncapped hold would be 1s.
        sched.note_receive_timeout(target, 0);
        sched.note_receive_timeout(target, 0);
        assert_eq!(sched.receive_hold_ms(target, 10_000), 1_000);

        // Activity within 3s caps at 250ms; outside the window it lifts again.
        sched.note_user_activity(10_000);
        assert_eq!(sched.receive_hold_ms(target, 12_000), 250);
        assert_eq!(sched.receive_hold_ms(target, 13_001), 1_000);
    }

    #[test]
    fn cancel_requested_caps_hold_at_half_second() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        // Would otherwise be 1s after two empty polls.
        sched.note_receive_timeout(target, 0);
        sched.note_receive_timeout(target, 0);
        sched.note_cancel_requested(id, 1_000);
        // Cancel resets the streak, so hold is base (250ms), still under the cap.
        assert_eq!(sched.receive_hold_ms(target, 1_000), 250);
        // Grow it back past the cap; the cancel cap holds it at 500ms.
        sched.note_receive_timeout(target, 1_000);
        sched.note_receive_timeout(target, 1_000);
        sched.note_receive_timeout(target, 1_000);
        assert_eq!(sched.receive_hold_ms(target, 1_000), 500);
    }

    #[test]
    fn next_eligible_is_none_for_finished_targets() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        assert_eq!(sched.next_eligible_at_ms(target), Some(0));
        assert!(sched.is_allowed_target(target, 0));
        sched.note_pipeline_finished(id, 1_000);
        assert_eq!(sched.next_eligible_at_ms(target), None);
        assert!(!sched.is_allowed_target(target, 1_000));
    }
}
