use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerStatusSnapshot {
    pub captured_at_unix_ms: u64,
    pub heartbeat_at_unix_ms: u64,
    pub lifecycle: ServerLifecyclePhase,
    pub current_command: Option<ServerCommandStatus>,
    pub current_work: Option<ServerWorkStatus>,
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
    pub placement: ServerPlacementStatus,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerWorkStatus {
    pub kind: String,
    pub stage: String,
    pub summary: String,
    pub started_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerCommandKind {
    Show,
    Hide,
    ChangeCharacter,
    PreviewTarget,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerPlacementStatus {
    pub mode: PlacementMode,
    pub anchor_policy: PlacementAnchorPolicy,
    pub selected_anchor_kind: PlacementAnchorKind,
    pub shared_visual_size_policy: SharedVisualSizePolicy,
    pub shared_visual_size_px: Option<VisualSizePx>,
    pub shared_anchor_positions: Option<PlacementAnchorPositions>,
    pub anchor_plan: PlacementAnchorPlan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementAnchorPlan {
    pub placement_mode: PlacementMode,
    pub anchor_policy: PlacementAnchorPolicy,
    pub selected_anchor_kind: PlacementAnchorKind,
    pub target_scope: PlacementTargetScope,
    pub target_count: usize,
    pub screen_safe_rect: ScreenRectPx,
    pub right_overflow_tolerance_px: f32,
    pub max_right_overflow_px: f32,
    pub targets: Vec<PlacementAnchorPlanTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementAnchorPlanTarget {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub scale: f32,
    pub visible_size_px: VisualSizePx,
    pub bottom_center_anchor_position: [f32; 2],
    pub bottom_right_anchor_position: [f32; 2],
    pub projected_visible_right_px: f32,
    pub right_limit_px: f32,
    pub right_overflow_px: f32,
    pub overflows_right: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenRectPx {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualSizePx {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlacementAnchorPositions {
    pub bottom_center: [f32; 2],
    pub bottom_right: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementMode {
    PerPsd,
    SharedVisualSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementAnchorPolicy {
    AdaptiveRightOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementAnchorKind {
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementTargetScope {
    CurrentPsd,
    CandidatePsdSet,
    FavoriteEnsemble,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedVisualSizePolicy {
    Height,
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
            current_work: None,
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
            placement: ServerPlacementStatus::default(),
            last_error: None,
        }
    }
}

impl ServerWorkStatus {
    pub fn started(
        kind: impl Into<String>,
        stage: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let now = now_unix_ms();
        Self {
            kind: kind.into(),
            stage: stage.into(),
            summary: summary.into(),
            started_at_unix_ms: now,
            updated_at_unix_ms: now,
        }
    }

    pub fn with_stage(&self, stage: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            kind: self.kind.clone(),
            stage: stage.into(),
            summary: summary.into(),
            started_at_unix_ms: self.started_at_unix_ms,
            updated_at_unix_ms: now_unix_ms(),
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

impl Default for ServerPlacementStatus {
    fn default() -> Self {
        Self {
            mode: PlacementMode::PerPsd,
            anchor_policy: PlacementAnchorPolicy::AdaptiveRightOverflow,
            selected_anchor_kind: PlacementAnchorKind::BottomCenter,
            shared_visual_size_policy: SharedVisualSizePolicy::Height,
            shared_visual_size_px: None,
            shared_anchor_positions: None,
            anchor_plan: PlacementAnchorPlan::default(),
        }
    }
}

impl Default for PlacementAnchorPlan {
    fn default() -> Self {
        Self {
            placement_mode: PlacementMode::PerPsd,
            anchor_policy: PlacementAnchorPolicy::AdaptiveRightOverflow,
            selected_anchor_kind: PlacementAnchorKind::BottomCenter,
            target_scope: PlacementTargetScope::Unknown,
            target_count: 0,
            screen_safe_rect: ScreenRectPx::default(),
            right_overflow_tolerance_px: 1.0,
            max_right_overflow_px: 0.0,
            targets: Vec::new(),
        }
    }
}

impl Default for ScreenRectPx {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
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
