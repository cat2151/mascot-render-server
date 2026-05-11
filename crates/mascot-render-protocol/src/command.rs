use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeCharacterRequest {
    pub character_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VptEnsembleRequest {
    pub character_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewTargetRequest {
    pub png_path: PathBuf,
    pub scale: Option<f32>,
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub display_diff_path: Option<PathBuf>,
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

pub fn validate_vpt_ensemble_request(request: &VptEnsembleRequest) -> Result<()> {
    if request.character_names.is_empty() {
        bail!("vpt ensemble character_names must contain at least one character");
    }

    for (index, character_name) in request.character_names.iter().enumerate() {
        if character_name.trim().is_empty() {
            bail!("vpt ensemble character_names[{index}] must not be empty");
        }
    }
    Ok(())
}

pub fn validate_preview_target_request(request: &PreviewTargetRequest) -> Result<()> {
    validate_non_empty_path(&request.png_path, "png_path")?;
    validate_non_empty_path(&request.zip_path, "zip_path")?;
    validate_non_empty_path(&request.psd_path_in_zip, "psd_path_in_zip")?;
    if let Some(display_diff_path) = request.display_diff_path.as_ref() {
        validate_non_empty_path(display_diff_path, "display_diff_path")?;
    }
    if let Some(scale) = request.scale {
        if !scale.is_finite() {
            bail!("preview target scale must be finite");
        }
        if scale <= 0.0 {
            bail!("preview target scale must be greater than zero");
        }
    }
    Ok(())
}

fn validate_non_empty_path(path: &Path, label: &str) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("preview target {label} must not be empty");
    }
    Ok(())
}
