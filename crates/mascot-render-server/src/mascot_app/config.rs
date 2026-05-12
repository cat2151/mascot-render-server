use std::time::SystemTime;

use mascot_render_core::MascotConfig;
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, PlacementMode};

use super::effective_scale;

#[derive(Clone, Copy)]
pub(super) struct ReloadInputs {
    pub(super) config_modified_at: Option<SystemTime>,
    pub(super) runtime_state_modified_at: Option<SystemTime>,
    pub(super) favorite_ensemble_modified_at: Option<SystemTime>,
    pub(super) psd_viewer_tui_activity_modified_at: Option<SystemTime>,
    pub(super) window_history_modified_at: Option<SystemTime>,
}

pub(super) fn describe_motion_timeline_request(request: &MotionTimelineRequest) -> String {
    let mut shake_steps = 0usize;
    let mut mouth_flap_steps = 0usize;
    let target = request.target_character_name.as_deref().unwrap_or("-");

    for step in &request.steps {
        match step.kind {
            MotionTimelineKind::Shake => shake_steps += 1,
            MotionTimelineKind::MouthFlap => mouth_flap_steps += 1,
        }
    }

    if mouth_flap_steps > 0 && shake_steps == 0 {
        format!(
            "口パクしました: steps={} mouth_flap_steps={} target_character_name={target}",
            request.steps.len(),
            mouth_flap_steps
        )
    } else if shake_steps > 0 && mouth_flap_steps == 0 {
        format!(
            "揺れモーションを開始しました: steps={} shake_steps={} target_character_name={target}",
            request.steps.len(),
            shake_steps
        )
    } else {
        format!(
            "モーションタイムラインを開始しました: steps={} shake_steps={} mouth_flap_steps={} target_character_name={target}",
            request.steps.len(),
            shake_steps,
            mouth_flap_steps
        )
    }
}

pub(super) fn active_config_scale(config: &MascotConfig) -> Option<f32> {
    if config.ensemble_mode.is_ensemble() {
        config.ensemble_scale
    } else {
        config.scale
    }
}

pub(super) fn active_display_scale(config: &MascotConfig, width: u32, height: u32) -> f32 {
    if config.ensemble_mode.is_ensemble() {
        config.ensemble_scale.unwrap_or(1.0)
    } else {
        effective_scale(width, height, config.scale)
    }
}

pub(super) fn should_reload_config(current: ReloadInputs, next: ReloadInputs) -> bool {
    current.config_modified_at != next.config_modified_at
        || current.runtime_state_modified_at != next.runtime_state_modified_at
        || current.favorite_ensemble_modified_at != next.favorite_ensemble_modified_at
        || current.psd_viewer_tui_activity_modified_at != next.psd_viewer_tui_activity_modified_at
        || current.window_history_modified_at != next.window_history_modified_at
}

pub(super) fn should_restore_window_history_for_reload(
    placement_mode: PlacementMode,
    history_path_changed: bool,
    window_history_file_changed: bool,
) -> bool {
    placement_mode == PlacementMode::PerPsd && (history_path_changed || window_history_file_changed)
}

pub(super) fn should_refresh_auxiliary_skins_now(
    config_reloaded_this_frame: bool,
    pending_auxiliary_skin_refresh: bool,
) -> bool {
    pending_auxiliary_skin_refresh && !config_reloaded_this_frame
}

#[cfg(test)]
pub(crate) fn should_reload_config_for_test(
    current: [Option<SystemTime>; 5],
    next: [Option<SystemTime>; 5],
) -> bool {
    should_reload_config(
        ReloadInputs {
            config_modified_at: current[0],
            runtime_state_modified_at: current[1],
            favorite_ensemble_modified_at: current[2],
            psd_viewer_tui_activity_modified_at: current[3],
            window_history_modified_at: current[4],
        },
        ReloadInputs {
            config_modified_at: next[0],
            runtime_state_modified_at: next[1],
            favorite_ensemble_modified_at: next[2],
            psd_viewer_tui_activity_modified_at: next[3],
            window_history_modified_at: next[4],
        },
    )
}

#[cfg(test)]
pub(crate) fn should_refresh_auxiliary_skins_now_for_test(
    config_reloaded_this_frame: bool,
    pending_auxiliary_skin_refresh: bool,
) -> bool {
    should_refresh_auxiliary_skins_now(config_reloaded_this_frame, pending_auxiliary_skin_refresh)
}

#[cfg(test)]
pub(crate) fn should_restore_window_history_for_reload_for_test(
    placement_mode: PlacementMode,
    history_path_changed: bool,
    window_history_file_changed: bool,
) -> bool {
    should_restore_window_history_for_reload(
        placement_mode,
        history_path_changed,
        window_history_file_changed,
    )
}
