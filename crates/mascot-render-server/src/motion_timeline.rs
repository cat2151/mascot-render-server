use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest};
use mascot_render_core::MotionState;

use crate::MascotWindowLayout;

pub fn validate_motion_timeline_request(request: &MotionTimelineRequest) -> Result<()> {
    if request.steps.len() != 1 {
        bail!(
            "motion timeline must contain exactly one step, got {}",
            request.steps.len()
        );
    }

    let step = &request.steps[0];
    if step.duration_ms == 0 {
        bail!("motion timeline duration must be greater than zero");
    }
    if step.fps == 0 {
        bail!("motion timeline fps must be greater than zero");
    }

    match step.kind {
        MotionTimelineKind::Shake => Ok(()),
        MotionTimelineKind::MouthFlap => Ok(()),
    }
}

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
