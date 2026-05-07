use std::path::{Path, PathBuf};

use anyhow::Result;
use eframe::egui::{Pos2, Vec2};
use mascot_render_control::{log_server_error, log_server_info, log_server_skin_info};
use mascot_render_protocol::{
    PlacementAnchorKind, PlacementAnchorPositions, PlacementMode, VisualSizePx,
};
use mascot_render_server::{anchored_inner_origin_for_kind, MascotWindowLayout};

use super::MascotApp;
use mascot_render_server::window_history::ViewportInfo;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ScaleChangeTrigger {
    Keyboard,
    MouseWheel { raw_scroll_delta_y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScaleLayoutChange {
    pub selected_anchor_kind: PlacementAnchorKind,
    pub previous_layout: MascotWindowLayout,
    pub next_layout: MascotWindowLayout,
    pub viewport_info: Option<ViewportInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefreshWindowLayoutDiagnostics {
    pub selected_anchor_kind: PlacementAnchorKind,
    pub previous_layout: MascotWindowLayout,
    pub next_layout: MascotWindowLayout,
    pub viewport_info: Option<ViewportInfo>,
    pub preserved_anchor_position: Option<Pos2>,
    pub next_inner_origin: Option<Pos2>,
    pub next_outer_position: Option<Pos2>,
}

pub(crate) fn change_character_stage_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    control_command_stage_message("change_character", previous_png_path, png_path, stage)
}

pub(crate) fn preview_target_stage_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    control_command_stage_message("preview_target", previous_png_path, png_path, stage)
}

fn control_command_stage_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    format!(
        "trigger=control_command action={action} character変更を処理中です: stage={stage} from={} to={}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn change_character_success_message(
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    control_command_success_message(
        "change_character",
        previous_png_path,
        png_path,
        runtime_state_path,
        persisted_png_path,
    )
}

pub(crate) fn preview_target_success_message(
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    control_command_success_message(
        "preview_target",
        previous_png_path,
        png_path,
        runtime_state_path,
        persisted_png_path,
    )
}

fn control_command_success_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    format!(
        "trigger=control_command action={action} character変更に成功しました: from={} to={} runtime_state_path={} persisted_png_path={}",
        previous_png_path.display(),
        png_path.display(),
        runtime_state_path.display(),
        persisted_png_path.display()
    )
}

pub(crate) fn change_character_failure_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    control_command_failure_message(
        "change_character",
        previous_png_path,
        png_path,
        stage,
        error_detail,
    )
}

pub(crate) fn preview_target_failure_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    control_command_failure_message(
        "preview_target",
        previous_png_path,
        png_path,
        stage,
        error_detail,
    )
}

fn control_command_failure_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    format!(
        "trigger=control_command action={action} character変更に失敗しました: stage={stage} from={} to={} error={error_detail}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn rendered_skin_message(png_path: &Path) -> String {
    let png_file_name = match png_path.file_name() {
        Some(file_name) => file_name.to_string_lossy().into_owned(),
        None => png_path.display().to_string(),
    };
    format!(
        "trigger=render action=display_skin displayed_png_path={} displayed_png_file_name={png_file_name}",
        png_path.display()
    )
}

pub(crate) fn scale_change_message(
    trigger: ScaleChangeTrigger,
    steps: i32,
    previous_scale: f32,
    next_scale: f32,
    favorite_ensemble_enabled: bool,
    png_path: &Path,
) -> String {
    let (trigger_name, raw_scroll_delta_y) = match trigger {
        ScaleChangeTrigger::Keyboard => ("keyboard", None),
        ScaleChangeTrigger::MouseWheel { raw_scroll_delta_y } => {
            ("mouse_wheel", Some(raw_scroll_delta_y))
        }
    };
    let raw_scroll_delta_y = raw_scroll_delta_y
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "trigger={trigger_name} action=change_scale scale変更を適用しました: steps={steps} previous_scale={} next_scale={} raw_scroll_delta_y={raw_scroll_delta_y} favorite_ensemble_enabled={favorite_ensemble_enabled} configured_png_path={}",
        scale_text(previous_scale),
        scale_text(next_scale),
        png_path.display()
    )
}

pub(crate) fn scale_layout_change_message(
    trigger: ScaleChangeTrigger,
    steps: i32,
    previous_scale: f32,
    next_scale: f32,
    layout_change: ScaleLayoutChange,
    png_path: &Path,
) -> String {
    let (trigger_name, raw_scroll_delta_y) = match trigger {
        ScaleChangeTrigger::Keyboard => ("keyboard", None),
        ScaleChangeTrigger::MouseWheel { raw_scroll_delta_y } => {
            ("mouse_wheel", Some(raw_scroll_delta_y))
        }
    };
    let raw_scroll_delta_y = raw_scroll_delta_y
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string());
    let previous_window_size = layout_change.previous_layout.window_size();
    let next_window_size = layout_change.next_layout.window_size();
    let previous_anchor_offset = layout_change
        .previous_layout
        .anchor_offset_for_kind(layout_change.selected_anchor_kind);
    let next_anchor_offset = layout_change
        .next_layout
        .anchor_offset_for_kind(layout_change.selected_anchor_kind);
    let previous_inner_origin = layout_change.viewport_info.map(|value| value.inner_origin);
    let previous_inner_to_outer_offset = layout_change
        .viewport_info
        .map(|value| value.inner_to_outer_offset);
    let next_inner_origin = layout_change.viewport_info.map(|value| {
        anchored_inner_origin_for_kind(
            value.inner_origin,
            layout_change.previous_layout,
            layout_change.next_layout,
            layout_change.selected_anchor_kind,
        )
    });
    let next_outer_position = next_inner_origin
        .zip(previous_inner_to_outer_offset)
        .map(|(inner_origin, inner_to_outer_offset)| inner_origin - inner_to_outer_offset);
    format!(
        "trigger={trigger_name} action=change_scale_layout scale変更時のwindow再配置を計算しました: steps={steps} previous_scale={} next_scale={} raw_scroll_delta_y={raw_scroll_delta_y} selected_anchor_kind={:?} previous_window_size={} next_window_size={} previous_anchor_offset={} next_anchor_offset={} previous_inner_origin={} previous_inner_to_outer_offset={} next_inner_origin={} next_outer_position={} configured_png_path={}",
        scale_text(previous_scale),
        scale_text(next_scale),
        layout_change.selected_anchor_kind,
        format_vec2(previous_window_size),
        format_vec2(next_window_size),
        format_vec2(previous_anchor_offset),
        format_vec2(next_anchor_offset),
        optional_pos2_text(previous_inner_origin),
        optional_vec2_text(previous_inner_to_outer_offset),
        optional_pos2_text(next_inner_origin),
        optional_pos2_text(next_outer_position),
        png_path.display()
    )
}

pub(crate) fn reloaded_scale_message(
    previous_scale: f32,
    next_scale: f32,
    previous_config_scale: Option<f32>,
    reloaded_config_scale: Option<f32>,
    pending_persisted_scale: bool,
    runtime_state_path: &Path,
    config_path: &Path,
) -> String {
    format!(
        "trigger=hot_reload action=change_scale scale変更を再読込しました: previous_scale={} next_scale={} previous_config_scale={} reloaded_config_scale={} pending_persisted_scale={pending_persisted_scale} runtime_state_path={} config_path={}",
        scale_text(previous_scale),
        scale_text(next_scale),
        optional_scale_text(previous_config_scale),
        optional_scale_text(reloaded_config_scale),
        runtime_state_path.display(),
        config_path.display()
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hot_reload_context_message(
    config_file_changed: bool,
    runtime_state_changed: bool,
    favorite_ensemble_file_changed: bool,
    psd_viewer_tui_activity_changed: bool,
    window_history_file_changed: bool,
    png_changed: bool,
    scale_changed: bool,
    favorite_ensemble_changed: bool,
    ensemble_mode_changed: bool,
    blink_source_changed: bool,
    history_path_changed: bool,
    placement_mode: PlacementMode,
    selected_anchor_kind: PlacementAnchorKind,
    shared_visual_size_px: Option<VisualSizePx>,
    shared_anchor_positions: Option<PlacementAnchorPositions>,
    previous_png_path: &Path,
    next_png_path: &Path,
    previous_zip_path: &Path,
    next_zip_path: &Path,
    previous_psd_path_in_zip: &Path,
    next_psd_path_in_zip: &Path,
    previous_scale: Option<f32>,
    next_scale: Option<f32>,
    restored_window_position: Option<Pos2>,
) -> String {
    format!(
        "trigger=hot_reload action=reload_config hot reload入力を検出しました: config_file_changed={config_file_changed} runtime_state_changed={runtime_state_changed} favorite_ensemble_file_changed={favorite_ensemble_file_changed} psd_viewer_tui_activity_changed={psd_viewer_tui_activity_changed} window_history_file_changed={window_history_file_changed} png_changed={png_changed} scale_changed={scale_changed} favorite_ensemble_changed={favorite_ensemble_changed} ensemble_mode_changed={ensemble_mode_changed} blink_source_changed={blink_source_changed} history_path_changed={history_path_changed} placement_mode={placement_mode:?} selected_anchor_kind={selected_anchor_kind:?} shared_visual_size_px={} shared_anchor_positions={} previous_png_path={} next_png_path={} previous_zip_path={} next_zip_path={} previous_psd_path_in_zip={} next_psd_path_in_zip={} previous_config_scale={} next_config_scale={} restored_window_position={}",
        optional_visual_size_text(shared_visual_size_px),
        optional_anchor_positions_text(shared_anchor_positions),
        previous_png_path.display(),
        next_png_path.display(),
        previous_zip_path.display(),
        next_zip_path.display(),
        previous_psd_path_in_zip.display(),
        next_psd_path_in_zip.display(),
        optional_scale_text(previous_scale),
        optional_scale_text(next_scale),
        optional_pos2_text(restored_window_position),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn refresh_window_layout_message(
    trigger: &str,
    placement_mode: PlacementMode,
    shared_visual_size_px: Option<VisualSizePx>,
    shared_anchor_positions: Option<PlacementAnchorPositions>,
    configured_png_path: &Path,
    configured_zip_path: &Path,
    configured_psd_path_in_zip: &Path,
    diagnostics: RefreshWindowLayoutDiagnostics,
) -> String {
    let previous_window_size = diagnostics.previous_layout.window_size();
    let next_window_size = diagnostics.next_layout.window_size();
    let previous_anchor_offset = diagnostics
        .previous_layout
        .anchor_offset_for_kind(diagnostics.selected_anchor_kind);
    let next_anchor_offset = diagnostics
        .next_layout
        .anchor_offset_for_kind(diagnostics.selected_anchor_kind);
    let previous_inner_origin = diagnostics.viewport_info.map(|value| value.inner_origin);
    let previous_inner_to_outer_offset = diagnostics
        .viewport_info
        .map(|value| value.inner_to_outer_offset);
    format!(
        "trigger={trigger} action=refresh_window_layout window再配置を計算しました: placement_mode={placement_mode:?} selected_anchor_kind={:?} shared_visual_size_px={} shared_anchor_positions={} previous_window_size={} next_window_size={} previous_anchor_offset={} next_anchor_offset={} previous_inner_origin={} previous_inner_to_outer_offset={} preserved_anchor_position={} next_inner_origin={} next_outer_position={} configured_png_path={} configured_zip_path={} configured_psd_path_in_zip={}",
        diagnostics.selected_anchor_kind,
        optional_visual_size_text(shared_visual_size_px),
        optional_anchor_positions_text(shared_anchor_positions),
        format_vec2(previous_window_size),
        format_vec2(next_window_size),
        format_vec2(previous_anchor_offset),
        format_vec2(next_anchor_offset),
        optional_pos2_text(previous_inner_origin),
        optional_vec2_text(previous_inner_to_outer_offset),
        optional_pos2_text(diagnostics.preserved_anchor_position),
        optional_pos2_text(diagnostics.next_inner_origin),
        optional_pos2_text(diagnostics.next_outer_position),
        configured_png_path.display(),
        configured_zip_path.display(),
        configured_psd_path_in_zip.display(),
    )
}

pub(crate) fn run_change_character_stage<T>(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    log_server_info(change_character_stage_message(
        previous_png_path,
        png_path,
        stage,
    ));
    operation().map_err(|error| {
        log_server_error(change_character_failure_message(
            previous_png_path,
            png_path,
            stage,
            &format!("{error:#}"),
        ));
        error
    })
}

impl MascotApp {
    pub(super) fn log_scale_change(
        &self,
        trigger: ScaleChangeTrigger,
        steps: i32,
        previous_scale: f32,
        next_scale: f32,
    ) {
        log_server_info(scale_change_message(
            trigger,
            steps,
            previous_scale,
            next_scale,
            self.config.favorite_ensemble_enabled,
            &self.config.png_path,
        ));
    }

    pub(super) fn log_scale_layout_change(
        &self,
        trigger: ScaleChangeTrigger,
        steps: i32,
        previous_scale: f32,
        next_scale: f32,
        previous_layout: MascotWindowLayout,
        viewport_info: Option<ViewportInfo>,
    ) {
        log_server_info(scale_layout_change_message(
            trigger,
            steps,
            previous_scale,
            next_scale,
            ScaleLayoutChange {
                selected_anchor_kind: self.placement_state.selected_anchor_kind,
                previous_layout,
                next_layout: self.window_layout,
                viewport_info,
            },
            &self.config.png_path,
        ));
    }

    pub(super) fn log_reloaded_scale_change(
        &self,
        previous_scale: f32,
        next_scale: f32,
        previous_config_scale: Option<f32>,
        reloaded_config_scale: Option<f32>,
    ) {
        log_server_info(reloaded_scale_message(
            previous_scale,
            next_scale,
            previous_config_scale,
            reloaded_config_scale,
            self.pending_persisted_scale.is_some(),
            &self.runtime_state_path,
            &self.config_path,
        ));
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn log_hot_reload_context(
        &self,
        config_file_changed: bool,
        runtime_state_changed: bool,
        favorite_ensemble_file_changed: bool,
        psd_viewer_tui_activity_changed: bool,
        window_history_file_changed: bool,
        previous_png_path: &Path,
        previous_zip_path: &Path,
        previous_psd_path_in_zip: &Path,
        previous_scale: Option<f32>,
        png_changed: bool,
        scale_changed: bool,
        favorite_ensemble_changed: bool,
        ensemble_mode_changed: bool,
        blink_source_changed: bool,
        history_path_changed: bool,
        restored_window_position: Option<Pos2>,
    ) {
        log_server_info(hot_reload_context_message(
            config_file_changed,
            runtime_state_changed,
            favorite_ensemble_file_changed,
            psd_viewer_tui_activity_changed,
            window_history_file_changed,
            png_changed,
            scale_changed,
            favorite_ensemble_changed,
            ensemble_mode_changed,
            blink_source_changed,
            history_path_changed,
            self.placement_state.mode,
            self.placement_state.selected_anchor_kind,
            self.placement_state.shared_visual_size_px,
            self.placement_state.shared_anchor_positions,
            previous_png_path,
            &self.config.png_path,
            previous_zip_path,
            &self.config.zip_path,
            previous_psd_path_in_zip,
            &self.config.psd_path_in_zip,
            previous_scale,
            super::config::active_config_scale(&self.config),
            restored_window_position,
        ));
    }

    pub(super) fn log_refresh_window_layout(
        &self,
        trigger: &str,
        diagnostics: RefreshWindowLayoutDiagnostics,
    ) {
        log_server_info(refresh_window_layout_message(
            trigger,
            self.placement_state.mode,
            self.placement_state.shared_visual_size_px,
            self.placement_state.shared_anchor_positions,
            &self.config.png_path,
            &self.config.zip_path,
            &self.config.psd_path_in_zip,
            diagnostics,
        ));
    }

    pub(super) fn log_rendered_skin_if_changed(&mut self, png_path: &Path) {
        if !should_log_rendered_skin(self.last_logged_skin_path.as_deref(), png_path) {
            return;
        }
        self.last_logged_skin_path = Some(png_path.to_path_buf());
        log_server_skin_info(rendered_skin_message(png_path));
    }

    pub(super) fn clear_last_logged_skin_path(&mut self) {
        clear_rendered_skin_path(&mut self.last_logged_skin_path);
    }
}

pub(crate) fn should_log_rendered_skin(
    last_logged_skin_path: Option<&Path>,
    png_path: &Path,
) -> bool {
    last_logged_skin_path != Some(png_path)
}

#[cfg(test)]
pub(crate) fn record_rendered_skin_path(
    last_logged_skin_path: &mut Option<PathBuf>,
    png_path: &Path,
) -> bool {
    if !should_log_rendered_skin(last_logged_skin_path.as_deref(), png_path) {
        return false;
    }
    *last_logged_skin_path = Some(png_path.to_path_buf());
    true
}

pub(crate) fn clear_rendered_skin_path(last_logged_skin_path: &mut Option<PathBuf>) {
    *last_logged_skin_path = None;
}

fn scale_text(value: f32) -> String {
    format!("{value:.3}")
}

fn format_pos2(value: Pos2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

fn format_vec2(value: Vec2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

fn optional_pos2_text(value: Option<Pos2>) -> String {
    value.map(format_pos2).unwrap_or_else(|| "-".to_string())
}

fn optional_vec2_text(value: Option<Vec2>) -> String {
    value.map(format_vec2).unwrap_or_else(|| "-".to_string())
}

fn optional_scale_text(value: Option<f32>) -> String {
    value.map(scale_text).unwrap_or_else(|| "-".to_string())
}

fn optional_visual_size_text(value: Option<VisualSizePx>) -> String {
    value
        .map(|size| format!("{:.3}x{:.3}", size.width, size.height))
        .unwrap_or_else(|| "-".to_string())
}

fn optional_anchor_positions_text(value: Option<PlacementAnchorPositions>) -> String {
    value
        .map(|positions| {
            format!(
                "bottom_center:{}|bottom_right:{}",
                format_pair(positions.bottom_center),
                format_pair(positions.bottom_right)
            )
        })
        .unwrap_or_else(|| "-".to_string())
}

fn format_pair([x, y]: [f32; 2]) -> String {
    format!("{x:.3},{y:.3}")
}

#[cfg(test)]
pub(crate) use change_character_failure_message as change_character_failure_message_for_test;
#[cfg(test)]
pub(crate) use change_character_stage_message as change_character_stage_message_for_test;
#[cfg(test)]
pub(crate) use change_character_success_message as change_character_success_message_for_test;
#[cfg(test)]
pub(crate) use clear_rendered_skin_path as clear_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use hot_reload_context_message as hot_reload_context_message_for_test;
#[cfg(test)]
pub(crate) use record_rendered_skin_path as record_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use refresh_window_layout_message as refresh_window_layout_message_for_test;
#[cfg(test)]
pub(crate) use reloaded_scale_message as reloaded_scale_message_for_test;
#[cfg(test)]
pub(crate) use rendered_skin_message as rendered_skin_message_for_test;
#[cfg(test)]
pub(crate) use scale_change_message as scale_change_message_for_test;
#[cfg(test)]
pub(crate) use scale_layout_change_message as scale_layout_change_message_for_test;
#[cfg(test)]
pub(crate) use should_log_rendered_skin as should_log_rendered_skin_for_test;
#[cfg(test)]
pub(crate) use RefreshWindowLayoutDiagnostics as RefreshWindowLayoutDiagnosticsForTest;
#[cfg(test)]
pub(crate) use ScaleChangeTrigger as ScaleChangeTriggerForTest;
#[cfg(test)]
pub(crate) use ScaleLayoutChange as ScaleLayoutChangeForTest;
