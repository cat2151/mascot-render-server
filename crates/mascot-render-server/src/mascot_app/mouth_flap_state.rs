use std::time::Instant;

use mascot_render_core::MotionState;

pub(super) fn mouth_flap_skin_state(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    now: Instant,
) -> Option<bool> {
    has_mouth_flap_skin
        .then(|| motion.mouth_flap_is_open(now))
        .flatten()
}

#[cfg(test)]
pub(crate) fn mouth_flap_skin_state_for_test(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    now: Instant,
) -> Option<bool> {
    mouth_flap_skin_state(has_mouth_flap_skin, motion, now)
}
