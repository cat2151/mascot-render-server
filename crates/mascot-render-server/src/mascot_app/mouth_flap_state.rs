use std::time::Instant;

use mascot_render_core::MotionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveSkinState {
    MouthOpen,
    MouthClosed,
    BlinkClosed,
    Open,
}

pub(super) fn mouth_flap_skin_state(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    now: Instant,
) -> Option<bool> {
    has_mouth_flap_skin
        .then(|| motion.mouth_flap_is_open(now))
        .flatten()
}

pub(super) fn active_skin_state(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    blink_closed: bool,
    now: Instant,
) -> ActiveSkinState {
    match mouth_flap_skin_state(has_mouth_flap_skin, motion, now) {
        Some(true) => ActiveSkinState::MouthOpen,
        Some(false) => ActiveSkinState::MouthClosed,
        None if blink_closed => ActiveSkinState::BlinkClosed,
        None => ActiveSkinState::Open,
    }
}

#[cfg(test)]
pub(crate) fn mouth_flap_skin_state_for_test(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    now: Instant,
) -> Option<bool> {
    mouth_flap_skin_state(has_mouth_flap_skin, motion, now)
}

#[cfg(test)]
pub(crate) fn active_skin_state_for_test(
    has_mouth_flap_skin: bool,
    motion: &mut MotionState,
    blink_closed: bool,
    now: Instant,
) -> ActiveSkinState {
    active_skin_state(has_mouth_flap_skin, motion, blink_closed, now)
}
