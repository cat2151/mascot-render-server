use std::time::{Duration, Instant};

use anyhow::Result;
use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest};
use mascot_render_control::validate_motion_timeline_request;
use mascot_render_core::MotionState;

use crate::MascotWindowLayout;

pub fn apply_motion_timeline_request(
    motion: &mut MotionState,
    window_layout: MascotWindowLayout,
    now: Instant,
    request: MotionTimelineRequest,
) -> Result<()> {
    validate_motion_timeline_request(&request)?;
    let step = &request.steps[0];

    match step.kind {
        MotionTimelineKind::Shake => motion.trigger_shake(
            now,
            window_layout.shake_amplitude_px(),
            Duration::from_millis(step.duration_ms),
            step.fps,
        ),
        MotionTimelineKind::MouthFlap => {
            motion.trigger_mouth_flap(now, Duration::from_millis(step.duration_ms), step.fps)
        }
    }

    Ok(())
}
