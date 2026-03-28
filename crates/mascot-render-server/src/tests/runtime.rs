use std::time::{Duration, Instant};

use mascot_render_core::MotionState;

use crate::mascot_app::mouth_flap_skin_state_for_test;

#[test]
fn mouth_flap_skin_state_requires_available_skin_and_active_motion() {
    let now = Instant::now();
    let mut inactive_motion = MotionState::new();
    assert_eq!(
        mouth_flap_skin_state_for_test(true, &mut inactive_motion, now),
        None
    );

    let mut active_motion_without_skin = MotionState::new();
    active_motion_without_skin.trigger_mouth_flap(now, Duration::from_secs(5), 4);
    assert_eq!(
        mouth_flap_skin_state_for_test(false, &mut active_motion_without_skin, now),
        None
    );

    let mut active_motion_with_skin = MotionState::new();
    active_motion_with_skin.trigger_mouth_flap(now, Duration::from_secs(5), 4);
    assert_eq!(
        mouth_flap_skin_state_for_test(true, &mut active_motion_with_skin, now),
        Some(true)
    );
    assert_eq!(
        mouth_flap_skin_state_for_test(
            true,
            &mut active_motion_with_skin,
            now + Duration::from_millis(250)
        ),
        Some(false)
    );
    assert_eq!(
        mouth_flap_skin_state_for_test(
            true,
            &mut active_motion_with_skin,
            now + Duration::from_secs(5)
        ),
        None
    );
}
