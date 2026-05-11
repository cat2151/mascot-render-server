use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::log_server_error;
use mascot_render_core::{load_mascot_config, psd_viewer_tui_activity_path};
use mascot_render_server::window_history::{
    current_viewport_info, load_window_position, window_history_path, WindowHistoryTracker,
};

use super::config::{
    active_config_scale, active_display_scale, should_reload_config,
    should_restore_window_history_for_reload, ReloadInputs,
};
use super::{active_ensemble_path, layout, path_modified_at, size_vec, window_title, MascotApp};

impl MascotApp {
    pub(super) fn reload_config_if_needed(&mut self, ctx: &egui::Context) -> Result<bool> {
        let next_config_modified_at = path_modified_at(&self.config_path);
        let next_runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        let ensemble_path = active_ensemble_path(self.config.ensemble_mode);
        let next_favorite_ensemble_modified_at =
            ensemble_path.as_deref().and_then(path_modified_at);
        let next_psd_viewer_tui_activity_modified_at =
            path_modified_at(&psd_viewer_tui_activity_path(&self.config_path));
        let current_history_path = window_history_path(&self.config);
        let next_window_history_modified_at = path_modified_at(&current_history_path);
        if !should_reload_config(
            ReloadInputs {
                config_modified_at: self.config_modified_at,
                runtime_state_modified_at: self.runtime_state_modified_at,
                favorite_ensemble_modified_at: self.favorite_ensemble_modified_at,
                psd_viewer_tui_activity_modified_at: self.psd_viewer_tui_activity_modified_at,
                window_history_modified_at: self.window_history_modified_at,
            },
            ReloadInputs {
                config_modified_at: next_config_modified_at,
                runtime_state_modified_at: next_runtime_state_modified_at,
                favorite_ensemble_modified_at: next_favorite_ensemble_modified_at,
                psd_viewer_tui_activity_modified_at: next_psd_viewer_tui_activity_modified_at,
                window_history_modified_at: next_window_history_modified_at,
            },
        ) {
            return Ok(false);
        }

        let mut work = self.start_current_work(
            "reload_config_if_needed",
            "load_mascot_config",
            format!("config_path={}", self.config_path.display()),
        );
        let previous_layout = self.window_layout;
        let previous_png_path = self.config.png_path.clone();
        let previous_zip_path = self.config.zip_path.clone();
        let previous_psd_path_in_zip = self.config.psd_path_in_zip.clone();
        let config_file_changed = self.config_modified_at != next_config_modified_at;
        let runtime_state_changed =
            self.runtime_state_modified_at != next_runtime_state_modified_at;
        let psd_viewer_tui_activity_changed =
            self.psd_viewer_tui_activity_modified_at != next_psd_viewer_tui_activity_modified_at;
        let window_history_file_changed =
            self.window_history_modified_at != next_window_history_modified_at;
        let next_config = load_mascot_config(&self.config_path)
            .with_context(|| format!("failed to hot-reload {}", self.config_path.display()))?;
        let next_config_ensemble_path = active_ensemble_path(next_config.ensemble_mode);
        let next_config_ensemble_modified_at = next_config_ensemble_path
            .as_deref()
            .and_then(path_modified_at);
        let favorite_ensemble_file_changed =
            self.favorite_ensemble_modified_at != next_config_ensemble_modified_at;
        let favorite_ensemble_changed =
            self.favorite_ensemble_modified_at != next_config_ensemble_modified_at;
        self.config_modified_at = next_config_modified_at;
        self.runtime_state_modified_at = next_runtime_state_modified_at;
        self.favorite_ensemble_modified_at = next_config_ensemble_modified_at;
        self.psd_viewer_tui_activity_modified_at = next_psd_viewer_tui_activity_modified_at;

        let ensemble_mode_changed = self.config.ensemble_mode != next_config.ensemble_mode;
        let png_changed = self.config.png_path != next_config.png_path;
        let scale_changed = active_config_scale(&self.config) != active_config_scale(&next_config);
        let blink_source_changed = self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip
            || self.config.display_diff_path != next_config.display_diff_path;
        let history_path_changed = ensemble_mode_changed
            || self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip;

        let previous_scale = self.scale;
        let previous_config_scale = active_config_scale(&self.config);
        let reloaded_config_scale = active_config_scale(&next_config);
        self.config = next_config;
        self.motion
            .set_always_idle_sink_enabled(self.config.always_idle_sink_enabled, Instant::now());
        if let Some(favorite_ensemble) = &mut self.favorite_ensemble {
            favorite_ensemble
                .set_always_idle_sink_enabled(self.config.always_idle_sink_enabled, Instant::now());
        }
        if png_changed || ensemble_mode_changed {
            work.update_stage(
                "load_active_skin",
                format!("png_path={}", self.config.png_path.display()),
            );
            self.open_skin = self.load_active_skin(ctx)?;
        }

        if self.config.ensemble_mode.is_ensemble() {
            if ensemble_mode_changed || favorite_ensemble_changed {
                let ensemble_path = active_ensemble_path(self.config.ensemble_mode);
                work.update_stage(
                    "load_active_ensemble",
                    format!(
                        "ensemble_mode={:?} ensemble_path={}",
                        self.config.ensemble_mode,
                        ensemble_path
                            .as_deref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "-".to_string())
                    ),
                );
                self.favorite_ensemble = self.load_active_ensemble_scene(ctx)?;
            }
        } else if ensemble_mode_changed || png_changed {
            self.favorite_ensemble = None;
        }
        if ensemble_mode_changed || favorite_ensemble_changed || png_changed || scale_changed {
            self.scale = active_display_scale(
                &self.config,
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
            );
            self.base_size = size_vec(
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
                Some(self.scale),
            );
            if scale_changed {
                self.log_reloaded_scale_change(
                    previous_scale,
                    self.scale,
                    previous_config_scale,
                    reloaded_config_scale,
                );
            }
        }

        let mut restored_window_position = None;
        if ensemble_mode_changed || favorite_ensemble_changed || png_changed || blink_source_changed
        {
            self.queue_auxiliary_skin_refresh();
            ctx.request_repaint();
        }
        if history_path_changed || window_history_file_changed {
            let next_history_path = if history_path_changed {
                window_history_path(&self.config)
            } else {
                current_history_path
            };
            let restore_window_history = should_restore_window_history_for_reload(
                self.placement_state.mode,
                history_path_changed,
                window_history_file_changed,
            );
            let saved_window_position = if restore_window_history {
                match load_window_position(&next_history_path) {
                    Ok(saved_window_position) => saved_window_position,
                    Err(error) => {
                        log_server_error(format!(
                            "failed to load mascot window history {}: {error:#}",
                            next_history_path.display()
                        ));
                        None
                    }
                }
            } else {
                None
            };
            self.window_history_modified_at = path_modified_at(&next_history_path);
            self.window_history =
                WindowHistoryTracker::new(next_history_path, saved_window_position);
            restored_window_position = saved_window_position;
        }
        work.update_stage(
            "refresh_window_layout",
            format!(
                "png_changed={png_changed} ensemble_mode_changed={ensemble_mode_changed} favorite_ensemble_changed={favorite_ensemble_changed}"
            ),
        );
        let layout_diagnostics = self.refresh_window_layout(ctx, previous_layout);
        self.log_hot_reload_context(
            config_file_changed,
            runtime_state_changed,
            favorite_ensemble_file_changed,
            psd_viewer_tui_activity_changed,
            window_history_file_changed,
            &previous_png_path,
            &previous_zip_path,
            &previous_psd_path_in_zip,
            previous_config_scale,
            png_changed,
            scale_changed,
            favorite_ensemble_changed,
            ensemble_mode_changed,
            blink_source_changed,
            history_path_changed,
            restored_window_position,
        );
        self.log_refresh_window_layout("hot_reload", layout_diagnostics);
        if let Some(position) = restored_window_position {
            layout::restore_anchor_position(self, ctx, position);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title(
            &self.config,
            &self.config_path,
        )));
        Ok(true)
    }

    pub(super) fn sync_window_history(&mut self, ctx: &egui::Context, now: Instant) -> Result<()> {
        if let Some(viewport_info) = current_viewport_info(ctx) {
            let anchor_positions = mascot_render_server::anchor_positions_from_inner_origin(
                viewport_info.inner_origin,
                self.window_layout,
            );
            self.window_history.observe(
                viewport_info.inner_origin + self.window_layout.anchor_offset(),
                now,
            )?;
            self.observe_current_placement_anchor_positions(anchor_positions);
            self.window_history_modified_at = path_modified_at(self.window_history.path());
        }
        Ok(())
    }
}
