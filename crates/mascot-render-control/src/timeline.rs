use anyhow::{bail, Result};
use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest};

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
