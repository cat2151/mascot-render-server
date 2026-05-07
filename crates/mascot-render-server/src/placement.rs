use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use eframe::egui::{Pos2, Vec2};
use mascot_render_core::workspace_cache_root;
use mascot_render_protocol::{
    now_unix_ms, PlacementAnchorKind, PlacementAnchorPlan, PlacementAnchorPlanTarget,
    PlacementAnchorPolicy, PlacementAnchorPositions, PlacementMode, PlacementTargetScope,
    ScreenRectPx, ServerPlacementStatus, SharedVisualSizePolicy, VisualSizePx,
};
use serde::{Deserialize, Serialize};

use crate::{AlphaBounds, MascotWindowLayout};

const PLACEMENT_STATE_VERSION: u32 = 1;
pub const RIGHT_OVERFLOW_TOLERANCE_PX: f32 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PsdPlacementKey {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementState {
    pub version: u32,
    pub mode: PlacementMode,
    pub anchor_policy: PlacementAnchorPolicy,
    pub selected_anchor_kind: PlacementAnchorKind,
    pub shared_visual_size_policy: SharedVisualSizePolicy,
    pub shared_visual_size_px: Option<VisualSizePx>,
    pub shared_anchor_positions: Option<PlacementAnchorPositions>,
    pub psd_states: Vec<PsdPlacementState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PsdPlacementState {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub anchor_positions: Option<PlacementAnchorPositions>,
    pub scale: Option<f32>,
    pub visual_size_px: Option<VisualSizePx>,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementPlanTargetInput {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub scale: f32,
    pub visible_size_px: VisualSizePx,
    pub bottom_center_anchor_position: [f32; 2],
    pub bottom_right_anchor_position: [f32; 2],
    pub bottom_center_anchor_offset: [f32; 2],
    pub bottom_right_anchor_offset: [f32; 2],
}

impl PsdPlacementKey {
    pub fn new(zip_path: impl Into<PathBuf>, psd_path_in_zip: impl Into<PathBuf>) -> Self {
        Self {
            zip_path: zip_path.into(),
            psd_path_in_zip: psd_path_in_zip.into(),
        }
    }
}

impl PlacementState {
    pub fn sanitize(mut self) -> Result<Self> {
        if self.version != PLACEMENT_STATE_VERSION {
            bail!(
                "unsupported placement state version: expected {}, got {}",
                PLACEMENT_STATE_VERSION,
                self.version
            );
        }
        self.shared_visual_size_px = self.shared_visual_size_px.filter(valid_visual_size);
        self.shared_anchor_positions = self.shared_anchor_positions.filter(valid_anchor_positions);
        self.psd_states = dedup_sanitized_psd_states(self.psd_states);
        Ok(self)
    }

    pub fn status(&self, anchor_plan: PlacementAnchorPlan) -> ServerPlacementStatus {
        ServerPlacementStatus {
            mode: self.mode,
            anchor_policy: self.anchor_policy,
            selected_anchor_kind: self.selected_anchor_kind,
            shared_visual_size_policy: self.shared_visual_size_policy,
            shared_visual_size_px: self.shared_visual_size_px,
            shared_anchor_positions: self.shared_anchor_positions,
            anchor_plan,
        }
    }

    pub fn psd_state(&self, key: &PsdPlacementKey) -> Option<&PsdPlacementState> {
        self.psd_states.iter().find(|state| state.matches(key))
    }

    pub fn ensure_psd_state(
        &mut self,
        key: PsdPlacementKey,
        scale: f32,
        anchor_positions: PlacementAnchorPositions,
        visual_size_px: VisualSizePx,
    ) -> bool {
        if self.psd_state(&key).is_some() {
            return false;
        }
        self.psd_states.push(PsdPlacementState {
            zip_path: key.zip_path,
            psd_path_in_zip: key.psd_path_in_zip,
            anchor_positions: Some(anchor_positions),
            scale: valid_scale(scale).then_some(scale),
            visual_size_px: valid_visual_size(&visual_size_px).then_some(visual_size_px),
            updated_at_unix_ms: now_unix_ms(),
        });
        true
    }

    pub fn update_psd_anchor_positions(
        &mut self,
        key: PsdPlacementKey,
        anchor_positions: PlacementAnchorPositions,
    ) -> bool {
        let Some(anchor_positions) =
            valid_anchor_positions(&anchor_positions).then_some(anchor_positions)
        else {
            return false;
        };
        let now = now_unix_ms();
        match self.psd_state_mut_or_insert(key) {
            Some(state)
                if !state
                    .anchor_positions
                    .is_some_and(|current| same_anchor_positions(current, anchor_positions)) =>
            {
                state.anchor_positions = Some(anchor_positions);
                state.updated_at_unix_ms = now;
                true
            }
            Some(_) => false,
            None => false,
        }
    }

    pub fn update_psd_scale(
        &mut self,
        key: PsdPlacementKey,
        scale: f32,
        visual_size_px: VisualSizePx,
    ) -> bool {
        if !valid_scale(scale) {
            return false;
        }
        let visual_size_px = valid_visual_size(&visual_size_px).then_some(visual_size_px);
        let now = now_unix_ms();
        match self.psd_state_mut_or_insert(key) {
            Some(state) if state.scale != Some(scale) || state.visual_size_px != visual_size_px => {
                state.scale = Some(scale);
                state.visual_size_px = visual_size_px;
                state.updated_at_unix_ms = now;
                true
            }
            Some(_) => false,
            None => false,
        }
    }

    pub fn update_shared_anchor_positions(
        &mut self,
        anchor_positions: PlacementAnchorPositions,
    ) -> bool {
        if !valid_anchor_positions(&anchor_positions)
            || self
                .shared_anchor_positions
                .is_some_and(|current| same_anchor_positions(current, anchor_positions))
        {
            return false;
        }
        self.shared_anchor_positions = Some(anchor_positions);
        true
    }

    pub fn update_shared_visual_size(&mut self, visual_size_px: VisualSizePx) -> bool {
        if !valid_visual_size(&visual_size_px) || self.shared_visual_size_px == Some(visual_size_px)
        {
            return false;
        }
        self.shared_visual_size_px = Some(visual_size_px);
        true
    }

    pub fn clear_current_anchor_positions(&mut self, key: &PsdPlacementKey) -> bool {
        let Some(state) = self.psd_states.iter_mut().find(|state| state.matches(key)) else {
            return false;
        };
        let changed = state.anchor_positions.is_some();
        state.anchor_positions = None;
        if changed {
            state.updated_at_unix_ms = now_unix_ms();
        }
        changed
    }

    pub fn clear_current_scale(&mut self, key: &PsdPlacementKey) -> bool {
        let Some(state) = self.psd_states.iter_mut().find(|state| state.matches(key)) else {
            return false;
        };
        let changed = state.scale.is_some() || state.visual_size_px.is_some();
        state.scale = None;
        state.visual_size_px = None;
        if changed {
            state.updated_at_unix_ms = now_unix_ms();
        }
        changed
    }

    pub fn clear_shared_anchor_positions(&mut self) -> bool {
        let changed = self.shared_anchor_positions.is_some();
        self.shared_anchor_positions = None;
        changed
    }

    pub fn clear_shared_visual_size(&mut self) -> bool {
        let changed = self.shared_visual_size_px.is_some();
        self.shared_visual_size_px = None;
        changed
    }

    fn psd_state_mut_or_insert(&mut self, key: PsdPlacementKey) -> Option<&mut PsdPlacementState> {
        if let Some(index) = self.psd_states.iter().position(|state| state.matches(&key)) {
            return self.psd_states.get_mut(index);
        }
        self.psd_states.push(PsdPlacementState {
            zip_path: key.zip_path,
            psd_path_in_zip: key.psd_path_in_zip,
            anchor_positions: None,
            scale: None,
            visual_size_px: None,
            updated_at_unix_ms: now_unix_ms(),
        });
        self.psd_states.last_mut()
    }
}

impl Default for PlacementState {
    fn default() -> Self {
        Self {
            version: PLACEMENT_STATE_VERSION,
            mode: PlacementMode::PerPsd,
            anchor_policy: PlacementAnchorPolicy::AdaptiveRightOverflow,
            selected_anchor_kind: PlacementAnchorKind::BottomCenter,
            shared_visual_size_policy: SharedVisualSizePolicy::Height,
            shared_visual_size_px: None,
            shared_anchor_positions: None,
            psd_states: Vec::new(),
        }
    }
}

impl PsdPlacementState {
    fn matches(&self, key: &PsdPlacementKey) -> bool {
        self.zip_path == key.zip_path && self.psd_path_in_zip == key.psd_path_in_zip
    }
}

pub fn placement_state_path() -> PathBuf {
    workspace_cache_root().join("placement_state.json")
}

pub fn load_placement_state(path: &Path) -> Result<PlacementState> {
    if !path.exists() {
        return Ok(PlacementState::default());
    }
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read placement state {}", path.display()))?;
    let state: PlacementState = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse placement state {}", path.display()))?;
    state.sanitize()
}

pub fn save_placement_state(path: &Path, state: &PlacementState) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(state).context("failed to serialize placement state")?;
    fs::write(path, json)
        .with_context(|| format!("failed to write placement state {}", path.display()))
}

pub fn visual_size_px(content_bounds: AlphaBounds, scale: f32) -> Option<VisualSizePx> {
    valid_scale(scale).then_some(VisualSizePx {
        width: content_bounds.width() as f32 * scale,
        height: content_bounds.height() as f32 * scale,
    })
}

pub fn shared_height_scale(
    shared_visual_size_px: VisualSizePx,
    next_content_bounds: AlphaBounds,
    next_image_size: [u32; 2],
    min_scale: f32,
) -> Option<f32> {
    if !valid_visual_size(&shared_visual_size_px) {
        return None;
    }
    let visible_height = next_content_bounds.height().max(1) as f32;
    let fallback_height = next_image_size[1].max(1) as f32;
    let next_visible_height = if visible_height.is_finite() && visible_height > 0.0 {
        visible_height
    } else {
        fallback_height
    };
    Some((shared_visual_size_px.height / next_visible_height).max(min_scale))
}

pub fn anchor_positions_from_inner_origin(
    inner_origin: Pos2,
    layout: MascotWindowLayout,
) -> PlacementAnchorPositions {
    let bottom_center = inner_origin + layout.bottom_center_anchor_offset();
    let bottom_right = inner_origin + layout.bottom_right_anchor_offset();
    PlacementAnchorPositions {
        bottom_center: [bottom_center.x, bottom_center.y],
        bottom_right: [bottom_right.x, bottom_right.y],
    }
}

pub fn anchor_position(positions: PlacementAnchorPositions, kind: PlacementAnchorKind) -> Pos2 {
    let [x, y] = match kind {
        PlacementAnchorKind::BottomCenter => positions.bottom_center,
        PlacementAnchorKind::BottomRight => positions.bottom_right,
    };
    Pos2::new(x, y)
}

pub fn anchor_offset_array(offset: Vec2) -> [f32; 2] {
    [offset.x, offset.y]
}

pub fn build_anchor_plan(
    mode: PlacementMode,
    target_scope: PlacementTargetScope,
    screen_safe_rect: ScreenRectPx,
    targets: Vec<PlacementPlanTargetInput>,
) -> PlacementAnchorPlan {
    let mut plan_targets = Vec::with_capacity(targets.len());
    let mut max_right_overflow_px = 0.0_f32;
    let mut any_overflows = false;

    for target in targets {
        let projected_inner_origin_x =
            target.bottom_center_anchor_position[0] - target.bottom_center_anchor_offset[0];
        let projected_visible_right_px =
            projected_inner_origin_x + target.bottom_right_anchor_offset[0];
        let right_overflow_px = (projected_visible_right_px - screen_safe_rect.max_x).max(0.0);
        let overflows_right = right_overflow_px > RIGHT_OVERFLOW_TOLERANCE_PX;
        max_right_overflow_px = max_right_overflow_px.max(right_overflow_px);
        any_overflows |= overflows_right;
        plan_targets.push(PlacementAnchorPlanTarget {
            zip_path: target.zip_path,
            psd_path_in_zip: target.psd_path_in_zip,
            scale: target.scale,
            visible_size_px: target.visible_size_px,
            bottom_center_anchor_position: target.bottom_center_anchor_position,
            bottom_right_anchor_position: target.bottom_right_anchor_position,
            projected_visible_right_px,
            right_limit_px: screen_safe_rect.max_x,
            right_overflow_px,
            overflows_right,
        });
    }

    let selected_anchor_kind = if any_overflows {
        PlacementAnchorKind::BottomRight
    } else {
        PlacementAnchorKind::BottomCenter
    };
    let target_count = plan_targets.len();
    PlacementAnchorPlan {
        placement_mode: mode,
        anchor_policy: PlacementAnchorPolicy::AdaptiveRightOverflow,
        selected_anchor_kind,
        target_scope,
        target_count,
        screen_safe_rect,
        right_overflow_tolerance_px: RIGHT_OVERFLOW_TOLERANCE_PX,
        max_right_overflow_px,
        targets: plan_targets,
    }
}

pub fn clamp_zoomed_inner_origin_to_right_edge(
    previous_inner_origin: Pos2,
    previous_layout: MascotWindowLayout,
    next_layout: MascotWindowLayout,
    anchor_kind: PlacementAnchorKind,
    screen_safe_rect: ScreenRectPx,
) -> Option<Pos2> {
    let right_limit_px = screen_safe_rect.max_x;
    let previous_visible_right_px =
        previous_inner_origin.x + previous_layout.bottom_right_anchor_offset().x;
    let next_inner_origin = crate::anchored_inner_origin_for_kind(
        previous_inner_origin,
        previous_layout,
        next_layout,
        anchor_kind,
    );
    let next_visible_right_px = next_inner_origin.x + next_layout.bottom_right_anchor_offset().x;
    let previous_overflows =
        previous_visible_right_px > right_limit_px + RIGHT_OVERFLOW_TOLERANCE_PX;
    let next_overflows = next_visible_right_px > right_limit_px + RIGHT_OVERFLOW_TOLERANCE_PX;
    if previous_overflows || !next_overflows {
        return None;
    }

    Some(Pos2::new(
        next_inner_origin.x - (next_visible_right_px - right_limit_px),
        next_inner_origin.y,
    ))
}

fn dedup_sanitized_psd_states(states: Vec<PsdPlacementState>) -> Vec<PsdPlacementState> {
    let mut seen = HashSet::new();
    let mut sanitized = Vec::new();
    for mut state in states.into_iter().rev() {
        let key = (state.zip_path.clone(), state.psd_path_in_zip.clone());
        if !seen.insert(key) {
            continue;
        }
        state.scale = state.scale.filter(|scale| valid_scale(*scale));
        state.anchor_positions = state.anchor_positions.filter(valid_anchor_positions);
        state.visual_size_px = state.visual_size_px.filter(valid_visual_size);
        sanitized.push(state);
    }
    sanitized.reverse();
    sanitized
}

fn valid_scale(scale: f32) -> bool {
    scale.is_finite() && scale > 0.0
}

fn valid_visual_size(size: &VisualSizePx) -> bool {
    size.width.is_finite() && size.width > 0.0 && size.height.is_finite() && size.height > 0.0
}

fn valid_anchor_positions(positions: &PlacementAnchorPositions) -> bool {
    positions
        .bottom_center
        .into_iter()
        .chain(positions.bottom_right)
        .all(|value| value.is_finite())
}

fn same_anchor_positions(left: PlacementAnchorPositions, right: PlacementAnchorPositions) -> bool {
    left.bottom_center
        .into_iter()
        .chain(left.bottom_right)
        .zip(right.bottom_center.into_iter().chain(right.bottom_right))
        .all(|(left, right)| (left - right).abs() < 0.5)
}
