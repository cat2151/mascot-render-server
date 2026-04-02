use std::time::{Duration, Instant, SystemTime};

use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest};
use mascot_render_core::MascotConfig;

use super::effective_scale;

const EFFECTIVE_CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

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

    for step in &request.steps {
        match step.kind {
            MotionTimelineKind::Shake => shake_steps += 1,
            MotionTimelineKind::MouthFlap => mouth_flap_steps += 1,
        }
    }

    if mouth_flap_steps > 0 && shake_steps == 0 {
        format!(
            "口パクしました: steps={} mouth_flap_steps={}",
            request.steps.len(),
            mouth_flap_steps
        )
    } else if shake_steps > 0 && mouth_flap_steps == 0 {
        format!(
            "揺れモーションを開始しました: steps={} shake_steps={}",
            request.steps.len(),
            shake_steps
        )
    } else {
        format!(
            "モーションタイムラインを開始しました: steps={} shake_steps={} mouth_flap_steps={}",
            request.steps.len(),
            shake_steps,
            mouth_flap_steps
        )
    }
}

pub(super) fn active_config_scale(config: &MascotConfig) -> Option<f32> {
    if config.favorite_ensemble_enabled {
        config.favorite_ensemble_scale
    } else {
        config.scale
    }
}

pub(super) fn active_display_scale(config: &MascotConfig, width: u32, height: u32) -> f32 {
    if config.favorite_ensemble_enabled {
        config.favorite_ensemble_scale.unwrap_or(1.0)
    } else {
        effective_scale(width, height, config.scale)
    }
}

pub(super) fn should_reload_config(
    current: ReloadInputs,
    next: ReloadInputs,
    last_effective_config_check_at: Instant,
    now: Instant,
) -> bool {
    current.config_modified_at != next.config_modified_at
        || current.runtime_state_modified_at != next.runtime_state_modified_at
        || current.favorite_ensemble_modified_at != next.favorite_ensemble_modified_at
        || current.psd_viewer_tui_activity_modified_at != next.psd_viewer_tui_activity_modified_at
        || current.window_history_modified_at != next.window_history_modified_at
        || now.duration_since(last_effective_config_check_at) >= EFFECTIVE_CONFIG_POLL_INTERVAL
}

#[cfg(test)]
pub(crate) fn should_reload_config_for_test(
    current: [Option<SystemTime>; 5],
    next: [Option<SystemTime>; 5],
    last_effective_config_check_at: Instant,
    now: Instant,
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
        last_effective_config_check_at,
        now,
    )
}
