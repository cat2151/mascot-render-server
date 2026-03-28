use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::mascot_motion::{
    BounceAnimationConfig, HeadHitbox, IdleSinkAnimationConfig, SquashBounceAnimationConfig,
};
use crate::mascot_paths::unix_timestamp;

use super::{MascotTarget, MASCOT_RUNTIME_STATE_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct MascotStaticConfigFile {
    #[serde(skip_serializing)]
    pub(super) version: Option<u32>,
    pub(super) always_bouncing: bool,
    pub(super) transparent_background_click_through: bool,
    pub(super) flash_blue_background_on_transparent_input: bool,
    pub(super) head_hitbox: HeadHitbox,
    pub(super) bounce: BounceAnimationConfig,
    pub(super) squash_bounce: SquashBounceAnimationConfig,
    pub(super) always_idle_sink: IdleSinkAnimationConfig,
    #[serde(skip_serializing)]
    pub(super) updated_at: Option<u64>,
}

impl Default for MascotStaticConfigFile {
    fn default() -> Self {
        Self {
            version: None,
            always_bouncing: false,
            transparent_background_click_through: false,
            flash_blue_background_on_transparent_input: true,
            head_hitbox: HeadHitbox::default(),
            bounce: BounceAnimationConfig::default(),
            squash_bounce: SquashBounceAnimationConfig::default(),
            always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct MascotRuntimeStateFile {
    pub(super) version: u32,
    pub(super) png_path: PathBuf,
    pub(super) scale: Option<f32>,
    pub(super) zip_path: PathBuf,
    pub(super) psd_path_in_zip: PathBuf,
    pub(super) display_diff_path: Option<PathBuf>,
    pub(super) updated_at: u64,
}

impl Default for MascotRuntimeStateFile {
    fn default() -> Self {
        Self {
            version: MASCOT_RUNTIME_STATE_VERSION,
            png_path: PathBuf::new(),
            scale: None,
            zip_path: PathBuf::new(),
            psd_path_in_zip: PathBuf::new(),
            display_diff_path: None,
            updated_at: 0,
        }
    }
}

impl From<&MascotTarget> for MascotRuntimeStateFile {
    fn from(target: &MascotTarget) -> Self {
        Self {
            version: MASCOT_RUNTIME_STATE_VERSION,
            png_path: target.png_path.clone(),
            scale: target.scale,
            zip_path: target.zip_path.clone(),
            psd_path_in_zip: target.psd_path_in_zip.clone(),
            display_diff_path: target.display_diff_path.clone(),
            updated_at: unix_timestamp(),
        }
    }
}

impl MascotRuntimeStateFile {
    pub(super) fn into_target(self) -> MascotTarget {
        MascotTarget {
            png_path: self.png_path,
            scale: self.scale,
            zip_path: self.zip_path,
            psd_path_in_zip: self.psd_path_in_zip,
            display_diff_path: self.display_diff_path,
        }
    }
}
