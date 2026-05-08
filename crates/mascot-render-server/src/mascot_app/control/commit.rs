use std::path::Path;

use eframe::egui;
use mascot_render_server::MascotWindowLayout;

use super::super::logging::{change_character_stage_message, preview_target_stage_message};
use super::super::MascotApp;
use super::PreparedSkinChange;
use crate::app_support::path_modified_at;

impl MascotApp {
    pub(super) fn commit_character_change(
        &mut self,
        ctx: &egui::Context,
        previous_layout: MascotWindowLayout,
        previous_png_path: &Path,
        prepared: PreparedSkinChange,
    ) {
        let next_png_path = prepared.next_config.png_path.clone();
        self.config = prepared.next_config;
        self.open_skin = prepared.open_skin;
        self.closed_skin = prepared.closed_skin;
        self.closed_skin_unavailable = false;
        self.mouth_open_skin = prepared.mouth_open_skin;
        self.mouth_closed_skin = prepared.mouth_closed_skin;
        self.scale = prepared.placement.scale;
        self.base_size = prepared.placement.base_size;
        let anchor_position = prepared.placement.anchor_position;
        let anchor_kind = prepared.placement.anchor_kind;
        self.eye_blink.reset(std::time::Instant::now());
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        mascot_render_control::log_server_info(change_character_stage_message(
            previous_png_path,
            &next_png_path,
            "refresh_window_layout",
        ));
        self.refresh_window_layout(ctx, previous_layout);
        self.restore_anchor_position_for_kind(ctx, anchor_position, anchor_kind);
    }

    pub(super) fn commit_preview_target_change(
        &mut self,
        ctx: &egui::Context,
        previous_layout: MascotWindowLayout,
        previous_png_path: &Path,
        prepared: PreparedSkinChange,
    ) {
        let next_png_path = prepared.next_config.png_path.clone();
        self.config = prepared.next_config;
        self.open_skin = prepared.open_skin;
        self.closed_skin = prepared.closed_skin;
        self.closed_skin_unavailable = false;
        self.mouth_open_skin = prepared.mouth_open_skin;
        self.mouth_closed_skin = prepared.mouth_closed_skin;
        self.scale = prepared.placement.scale;
        self.base_size = prepared.placement.base_size;
        let anchor_position = prepared.placement.anchor_position;
        let anchor_kind = prepared.placement.anchor_kind;
        self.eye_blink.reset(std::time::Instant::now());
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        mascot_render_control::log_server_info(preview_target_stage_message(
            previous_png_path,
            &next_png_path,
            "refresh_window_layout",
        ));
        self.refresh_window_layout(ctx, previous_layout);
        self.restore_anchor_position_for_kind(ctx, anchor_position, anchor_kind);
    }
}
