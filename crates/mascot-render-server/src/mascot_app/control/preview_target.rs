use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_error, log_server_info};
use mascot_render_protocol::PreviewTargetRequest;

use super::super::logging::{
    optional_scale_text, preview_target_failure_message, preview_target_stage_message,
    preview_target_success_message,
};
use super::super::persistence::{
    persist_requested_character_change, verify_persisted_character_change,
};
use super::super::MascotApp;
use super::{elapsed_ms_since, PreparedSkinChange};

impl MascotApp {
    pub(super) fn apply_preview_target(
        &mut self,
        ctx: &egui::Context,
        request: &PreviewTargetRequest,
    ) -> Result<()> {
        if self.config.ensemble_mode.is_ensemble() {
            let message = format!(
                "trigger=control_command action=preview_target ensemble_mode={:?} png_path={} zip_path={} psd_path_in_zip={} のため preview target を適用できません",
                self.config.ensemble_mode,
                request.png_path.display(),
                request.zip_path.display(),
                request.psd_path_in_zip.display()
            );
            log_server_info(message);
            bail!(
                "ensemble_mode={:?}; cannot apply preview target while ensemble mode is active",
                self.config.ensemble_mode
            );
        }

        let previous_png_path = self.config.png_path.clone();
        self.save_current_placement_anchor_positions(ctx);
        self.save_current_placement_scale();
        log_server_info(format!(
            "trigger=control_command action=preview_target preview target 適用を開始しました: from={} to={} selected_zip={} selected_psd={} selected_display_diff={} requested_scale={}",
            previous_png_path.display(),
            request.png_path.display(),
            request.zip_path.display(),
            request.psd_path_in_zip.display(),
            super::optional_path_text(request.display_diff_path.as_deref()),
            optional_scale_text(request.scale),
        ));
        let previous_layout = self.window_layout;
        let prepared = self.prepare_preview_target_change(ctx, &previous_png_path, request)?;
        let persisted_png_path = prepared.persisted_png_path.clone();
        let commit_started_at = Instant::now();
        self.commit_preview_target_change(ctx, previous_layout, &previous_png_path, prepared);
        self.record_performance_stage("refresh_window_layout", elapsed_ms_since(commit_started_at));
        log_server_info(preview_target_success_message(
            &previous_png_path,
            &request.png_path,
            &self.runtime_state_path,
            &persisted_png_path,
        ));
        Ok(())
    }

    fn prepare_preview_target_change(
        &mut self,
        ctx: &egui::Context,
        previous_png_path: &Path,
        request: &PreviewTargetRequest,
    ) -> Result<PreparedSkinChange> {
        let mut next_config = self.config.clone();
        next_config.png_path = request.png_path.clone();
        next_config.scale = request.scale;
        next_config.ensemble_scale = None;
        next_config.zip_path = request.zip_path.clone();
        next_config.psd_path_in_zip = request.psd_path_in_zip.clone();
        next_config.display_diff_path = request.display_diff_path.clone();

        let open_skin = self.run_timed_preview_target_stage(
            previous_png_path,
            &next_config.png_path,
            "load_base_skin",
            |app| {
                app.load_skin(ctx, &next_config.png_path).with_context(|| {
                    format!(
                        "failed to load requested preview-target skin image {}",
                        next_config.png_path.display()
                    )
                })
            },
        )?;
        let placement = self.prepare_placement_for_preview_target(
            ctx,
            &next_config.zip_path,
            &next_config.psd_path_in_zip,
            &open_skin,
            request.scale,
        );
        next_config.scale = Some(placement.scale);
        log_server_info(format!(
            "trigger=control_command action=preview_target stage=placement_plan selected_zip={} selected_psd={} placement_mode={:?} selected_anchor_kind={:?} target_count={} max_right_overflow_px={} scale={}",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
            placement.anchor_plan.placement_mode,
            placement.anchor_kind,
            placement.anchor_plan.target_count,
            placement.anchor_plan.max_right_overflow_px,
            placement.scale,
        ));
        log_server_info(format!(
            "trigger=control_command action=preview_target stage=defer_auxiliary_skins selected_zip={} selected_psd={} reason=show_default_png_first",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
        ));
        log_server_info(format!(
            "trigger=control_command action=preview_target stage=defer_mouth_flap_skins selected_zip={} selected_psd={} reason=lazy_generate_on_timeline",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
        ));
        self.run_timed_preview_target_stage(
            previous_png_path,
            &next_config.png_path,
            "persist_runtime_state",
            |app| {
                persist_requested_character_change(&app.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to persist requested preview target to {}",
                            app.runtime_state_path.display()
                        )
                    },
                )
            },
        )?;
        let persisted = self.run_timed_preview_target_stage(
            previous_png_path,
            &next_config.png_path,
            "verify_runtime_state",
            |app| {
                verify_persisted_character_change(&app.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to verify requested preview target in {}",
                            app.runtime_state_path.display()
                        )
                    },
                )
            },
        )?;

        Ok(PreparedSkinChange {
            placement,
            next_config,
            open_skin,
            closed_skin: None,
            mouth_open_skin: None,
            mouth_closed_skin: None,
            persisted_png_path: persisted.png_path,
        })
    }

    fn run_timed_preview_target_stage<T>(
        &mut self,
        previous_png_path: &Path,
        png_path: &Path,
        stage: &'static str,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let started_at = Instant::now();
        let result =
            run_preview_target_stage(previous_png_path, png_path, stage, || operation(self));
        self.record_performance_stage(stage, elapsed_ms_since(started_at));
        result
    }
}

fn run_preview_target_stage<T>(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &'static str,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    log_server_info(preview_target_stage_message(
        previous_png_path,
        png_path,
        stage,
    ));
    operation().map_err(|error| {
        log_server_error(preview_target_failure_message(
            previous_png_path,
            png_path,
            stage,
            &format!("{error:#}"),
        ));
        error
    })
}
