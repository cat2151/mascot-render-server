use std::path::Path;

use eframe::egui::Pos2;
use mascot_render_control::log_server_info;
use mascot_render_core::MascotEnsembleMode;
use mascot_render_protocol::{PlacementAnchorPositions, PlacementMode, VisualSizePx};
use mascot_render_server::anchored_inner_origin_for_kind;

use super::super::MascotApp;
use super::formatting::{
    format_vec2, optional_anchor_positions_text, optional_pos2_text, optional_scale_text,
    optional_vec2_text, optional_visual_size_text, scale_text,
};
use super::{RefreshWindowLayoutDiagnostics, ScaleChangeTrigger, ScaleLayoutChange};

pub(crate) fn scale_change_message(
    trigger: ScaleChangeTrigger,
    steps: i32,
    previous_scale: f32,
    next_scale: f32,
    ensemble_mode: MascotEnsembleMode,
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
        "trigger={trigger_name} action=change_scale scale変更を適用しました: steps={steps} previous_scale={} next_scale={} raw_scroll_delta_y={raw_scroll_delta_y} ensemble_mode={ensemble_mode:?} configured_png_path={}",
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
    selected_anchor_kind: mascot_render_protocol::PlacementAnchorKind,
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

impl MascotApp {
    pub(in crate::mascot_app) fn log_scale_change(
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
            self.config.ensemble_mode,
            &self.config.png_path,
        ));
    }

    pub(in crate::mascot_app) fn log_scale_layout_change(
        &self,
        trigger: ScaleChangeTrigger,
        steps: i32,
        previous_scale: f32,
        next_scale: f32,
        previous_layout: mascot_render_server::MascotWindowLayout,
        viewport_info: Option<mascot_render_server::window_history::ViewportInfo>,
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

    pub(in crate::mascot_app) fn log_reloaded_scale_change(
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
    pub(in crate::mascot_app) fn log_hot_reload_context(
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
            super::super::config::active_config_scale(&self.config),
            restored_window_position,
        ));
    }

    pub(in crate::mascot_app) fn log_refresh_window_layout(
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
}
