use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSkinRequest {
    pub png_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionTimelineKind {
    Shake,
    MouthFlap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionTimelineStep {
    pub kind: MotionTimelineKind,
    pub duration_ms: u64,
    pub fps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionTimelineRequest {
    pub steps: Vec<MotionTimelineStep>,
}

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
