use super::{AnimationKind, MotionState};

impl MotionState {
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        Self::with_seed_and_idle_phase_offset(seed, 0.0)
    }

    pub(crate) fn next_animation_name(&self) -> &'static str {
        match self.next_kind.unwrap_or(AnimationKind::Bounce) {
            AnimationKind::Bounce => "bounce",
            AnimationKind::SquashBounce => "squash_bounce",
            AnimationKind::IdleSink => "idle_sink",
            AnimationKind::Shake => "shake",
        }
    }
}
