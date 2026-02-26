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
    next_eligible_at_ms: u64,
    timeout_streak: u32,
    finished: bool,
    cancel_requested_at_ms: Option<u64>,
}

pub trait ReceiveScheduler {
    fn note_cancel_requested(&mut self, pipeline_id: Uuid, now_ms: u64);
    fn note_pipeline_finished(&mut self, pipeline_id: Uuid, now_ms: u64);
    fn note_receive_timeout(&mut self, target: TargetId, now_ms: u64);
    fn note_receive_progress(&mut self, target: TargetId, now_ms: u64);

    fn is_allowed_target(&self, target: TargetId, now_ms: u64) -> bool;
    /// Returns the earliest time this target may be polled, or `None` if the
    /// target is finished and should never be scheduled again.
    fn next_eligible_at_ms(&self, target: TargetId) -> Option<u64>;
}

/// Default serial scheduler policy:
/// - if a pipeline is finished, never poll it again
/// - after cancel is requested, keep polling (short slices) until a finish signal arrives
/// - apply exponential backoff after repeated receive timeouts per target
#[derive(Debug, Default)]
pub struct DefaultReceiveScheduler {
    targets: HashMap<TargetId, TargetState>,
    base_backoff_ms: u64,
    max_backoff_ms: u64,
    max_backoff_after_cancel_ms: u64,
}

impl DefaultReceiveScheduler {
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            base_backoff_ms: 200,
            max_backoff_ms: 5_000,
            max_backoff_after_cancel_ms: 1_000,
        }
    }

    fn state_mut(&mut self, id: TargetId) -> &mut TargetState {
        self.targets.entry(id).or_default()
    }

    fn state(&self, id: TargetId) -> Option<&TargetState> {
        self.targets.get(&id)
    }

    fn backoff_for_streak(&self, streak: u32) -> u64 {
        // backoff = base * 2^(streak-1), capped
        let exp = streak.saturating_sub(1).min(31);
        let pow = 1u64 << exp;
        (self.base_backoff_ms.saturating_mul(pow)).min(self.max_backoff_ms)
    }
}

impl ReceiveScheduler for DefaultReceiveScheduler {
    fn note_cancel_requested(&mut self, pipeline_id: Uuid, now_ms: u64) {
        let st = self.state_mut(TargetId::Pipeline(pipeline_id));
        // Cancellation is cooperative. Keep polling this target (short slices)
        // so we can observe PipelineFinished or a non-fatal InvalidSelectors
        // fault that will be translated into PipelineFinished by the backend.
        st.cancel_requested_at_ms = Some(now_ms);
        st.timeout_streak = 0;
        st.next_eligible_at_ms = now_ms;
    }

    fn note_pipeline_finished(&mut self, pipeline_id: Uuid, now_ms: u64) {
        let st = self.state_mut(TargetId::Pipeline(pipeline_id));
        st.finished = true;
        st.next_eligible_at_ms = now_ms;
    }

    fn note_receive_timeout(&mut self, target: TargetId, now_ms: u64) {
        let streak = {
            let st = self.state_mut(target);
            st.timeout_streak = st.timeout_streak.saturating_add(1);
            st.timeout_streak
        };

        let backoff = {
            let b = self.backoff_for_streak(streak);
            let st = self.state_mut(target);
            if st.cancel_requested_at_ms.is_some() {
                b.min(self.max_backoff_after_cancel_ms)
            } else {
                b
            }
        };
        let st = self.state_mut(target);
        st.next_eligible_at_ms = now_ms.saturating_add(backoff);
    }

    fn note_receive_progress(&mut self, target: TargetId, _now_ms: u64) {
        let st = self.state_mut(target);
        st.timeout_streak = 0;
        st.next_eligible_at_ms = 0;
    }

    fn is_allowed_target(&self, target: TargetId, now_ms: u64) -> bool {
        let Some(st) = self.state(target) else {
            return true;
        };
        if st.finished {
            return false;
        }
        now_ms >= st.next_eligible_at_ms
    }

    fn next_eligible_at_ms(&self, target: TargetId) -> Option<u64> {
        let Some(st) = self.state(target) else {
            return Some(0);
        };
        if st.finished {
            return None;
        }
        Some(st.next_eligible_at_ms)
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
    fn timeout_backoff_delays_polling_then_allows() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        // First timeout applies base backoff (200ms)
        sched.note_receive_timeout(target, 1_000);
        assert!(!sched.is_allowed_target(target, 1_050));
        assert!(sched.is_allowed_target(target, 1_250));
    }

    #[test]
    fn progress_resets_backoff() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        sched.note_receive_timeout(target, 1_000);
        assert!(!sched.is_allowed_target(target, 1_050));
        sched.note_receive_progress(target, 1_060);
        assert!(sched.is_allowed_target(target, 1_061));
    }

    #[test]
    fn next_eligible_is_none_for_cancelled_targets() {
        let mut sched = DefaultReceiveScheduler::new();
        let id = Uuid::new_v4();
        let target = TargetId::Pipeline(id);

        assert_eq!(sched.next_eligible_at_ms(target), Some(0));
        sched.note_pipeline_finished(id, 1_000);
        assert_eq!(sched.next_eligible_at_ms(target), None);
    }
}
