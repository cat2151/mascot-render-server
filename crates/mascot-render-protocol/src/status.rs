use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerStatusSnapshot {
    pub captured_at_unix_ms: u64,
    pub heartbeat_at_unix_ms: u64,
    pub lifecycle: ServerLifecyclePhase,
    pub current_command: Option<ServerCommandStatus>,
    pub last_completed_command: Option<ServerCommandStatus>,
    pub last_failed_command: Option<ServerCommandStatus>,
    pub configured_character_name: Option<String>,
    pub configured_png_path: PathBuf,
    pub configured_zip_path: PathBuf,
    pub configured_psd_path_in_zip: PathBuf,
    pub displayed_png_path: PathBuf,
    pub favorite_ensemble_enabled: bool,
    pub favorite_ensemble_loaded: bool,
    pub scale: f32,
    pub motion: ServerMotionStatus,
    pub window: ServerWindowStatus,
    pub config_path: PathBuf,
    pub runtime_state_path: PathBuf,
    pub pending_persisted_scale: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerLifecyclePhase {
    Starting,
    Running,
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerCommandStatus {
    pub kind: ServerCommandKind,
    pub stage: ServerCommandStage,
    pub summary: String,
    pub requested_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerCommandKind {
    Show,
    Hide,
    ChangeCharacter,
    Timeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerCommandStage {
    Queued,
    Applying,
    Applied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ServerMotionStatus {
    pub active: bool,
    pub blink_closed: bool,
    pub mouth_flap_open: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerWindowStatus {
    pub anchor_position: Option<[f32; 2]>,
    pub window_size: [f32; 2],
}

impl ServerStatusSnapshot {
    pub fn starting(
        config_path: PathBuf,
        runtime_state_path: PathBuf,
        configured_png_path: PathBuf,
        configured_zip_path: PathBuf,
        configured_psd_path_in_zip: PathBuf,
    ) -> Self {
        let now = now_unix_ms();
        Self {
            captured_at_unix_ms: now,
            heartbeat_at_unix_ms: now,
            lifecycle: ServerLifecyclePhase::Starting,
            current_command: None,
            last_completed_command: None,
            last_failed_command: None,
            configured_character_name: None,
            configured_png_path: configured_png_path.clone(),
            configured_zip_path,
            configured_psd_path_in_zip,
            displayed_png_path: configured_png_path,
            favorite_ensemble_enabled: false,
            favorite_ensemble_loaded: false,
            scale: 1.0,
            motion: ServerMotionStatus::default(),
            window: ServerWindowStatus::default(),
            config_path,
            runtime_state_path,
            pending_persisted_scale: false,
            last_error: None,
        }
    }
}

impl ServerCommandStatus {
    pub fn queued(kind: ServerCommandKind, summary: impl Into<String>) -> Self {
        let now = now_unix_ms();
        Self {
            kind,
            stage: ServerCommandStage::Queued,
            summary: summary.into(),
            requested_at_unix_ms: now,
            updated_at_unix_ms: now,
            error: None,
        }
    }

    pub fn with_stage(
        &self,
        stage: ServerCommandStage,
        updated_at_unix_ms: u64,
        error: Option<String>,
    ) -> Self {
        Self {
            kind: self.kind,
            stage,
            summary: self.summary.clone(),
            requested_at_unix_ms: self.requested_at_unix_ms,
            updated_at_unix_ms,
            error,
        }
    }
}

impl Default for ServerWindowStatus {
    fn default() -> Self {
        Self {
            anchor_position: None,
            window_size: [0.0, 0.0],
        }
    }
}

pub fn now_unix_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
