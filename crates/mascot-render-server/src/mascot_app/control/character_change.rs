use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::log_server_info;
use mascot_render_core::MascotConfig;

use super::super::character::{resolve_character_skin, ResolvedCharacterSkin};
use super::super::logging::{change_character_success_message, run_change_character_stage};
use super::super::persistence::{
    persist_requested_character_change, verify_persisted_character_change,
};
use super::super::MascotApp;
use super::{elapsed_ms_since, PreparedSkinChange};

impl MascotApp {
    pub(super) fn change_character(
        &mut self,
        ctx: &egui::Context,
        character_name: &str,
    ) -> Result<()> {
        if self.config.favorite_ensemble_enabled {
            let message = format!(
                "trigger=control_command action=change_character favorite_ensemble_enabled=true requested_character={} のため character変更できません",
                character_name
            );
            log_server_info(message);
            bail!("favorite_ensemble_enabled=true; cannot change character while favorite ensemble is active");
        }

        let resolve_started_at = Instant::now();
        let resolved = resolve_character_skin(&self.core, character_name).with_context(|| {
            format!(
                "failed to resolve requested character: requested_character={} current_png={} current_zip={} current_psd={} current_display_diff={}",
                character_name,
                self.config.png_path.display(),
                self.config.zip_path.display(),
                self.config.psd_path_in_zip.display(),
                super::optional_path_text(self.config.display_diff_path.as_deref())
            )
        });
        self.record_performance_stage(
            "resolve_character_skin",
            elapsed_ms_since(resolve_started_at),
        );
        let resolved = resolved?;
        log_server_info(format!(
            "trigger=control_command action=change_character requested_character={} candidate_count={} selected_zip={} selected_psd={} selected_png={} selected_display_diff={}",
            resolved.character_name,
            resolved.candidate_count,
            resolved.zip_path.display(),
            resolved.psd_path_in_zip.display(),
            resolved.png_path.display(),
            super::optional_path_text(resolved.display_diff_path.as_deref())
        ));

        if config_matches_resolved_character(&self.config, &resolved) {
            let verify_started_at = Instant::now();
            let verify_result = verify_persisted_character_change(&self.config_path, &self.config);
            self.record_performance_stage(
                "verify_current_runtime_state",
                elapsed_ms_since(verify_started_at),
            );
            match verify_result {
                Ok(persisted) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_character character変更をスキップしました: requested_character={} selected_png={} は現在の character source と同じで runtime state も一致しています runtime_state_path={} persisted_png_path={} persisted_zip={} persisted_psd={}",
                        resolved.character_name,
                        resolved.png_path.display(),
                        self.runtime_state_path.display(),
                        persisted.png_path.display(),
                        persisted.zip_path.display(),
                        persisted.psd_path_in_zip.display()
                    ));
                    return Ok(());
                }
                Err(error) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_character requested_character={} selected_png={} は現在の character source と同じですが runtime state の検証に失敗したため再試行します: runtime_state_path={} error={error:#}",
                        resolved.character_name,
                        resolved.png_path.display(),
                        self.runtime_state_path.display()
                    ));
                }
            }
        }

        let previous_png_path = self.config.png_path.clone();
        self.save_current_placement_anchor_positions(ctx);
        self.save_current_placement_scale();
        log_server_info(format!(
            "trigger=control_command action=change_character character変更を開始しました: requested_character={} from={} to={} selected_zip={} selected_psd={}",
            resolved.character_name,
            previous_png_path.display(),
            resolved.png_path.display(),
            resolved.zip_path.display(),
            resolved.psd_path_in_zip.display()
        ));
        let previous_layout = self.window_layout;
        let prepared = self.prepare_character_change(ctx, &previous_png_path, &resolved)?;
        let persisted_png_path = prepared.persisted_png_path.clone();
        let commit_started_at = Instant::now();
        self.commit_character_change(ctx, previous_layout, &previous_png_path, prepared);
        self.record_performance_stage("refresh_window_layout", elapsed_ms_since(commit_started_at));
        log_server_info(change_character_success_message(
            &previous_png_path,
            &resolved.png_path,
            &self.runtime_state_path,
            &persisted_png_path,
        ));
        Ok(())
    }

    fn prepare_character_change(
        &mut self,
        ctx: &egui::Context,
        previous_png_path: &Path,
        resolved: &ResolvedCharacterSkin,
    ) -> Result<PreparedSkinChange> {
        let mut next_config = self.config.clone();
        apply_resolved_character(&mut next_config, resolved);

        let open_skin = self.run_timed_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "load_base_skin",
            |app| {
                app.load_skin(ctx, &next_config.png_path).with_context(|| {
                    format!(
                        "failed to load requested mascot skin image {}",
                        next_config.png_path.display()
                    )
                })
            },
        )?;
        let placement = self.prepare_placement_for_character_change(ctx, resolved, &open_skin);
        next_config.scale = Some(placement.scale);
        log_server_info(format!(
            "trigger=control_command action=change_character stage=placement_plan selected_zip={} selected_psd={} placement_mode={:?} selected_anchor_kind={:?} target_count={} max_right_overflow_px={} scale={}",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
            placement.anchor_plan.placement_mode,
            placement.anchor_kind,
            placement.anchor_plan.target_count,
            placement.anchor_plan.max_right_overflow_px,
            placement.scale,
        ));
        log_server_info(format!(
            "trigger=control_command action=change_character stage=defer_auxiliary_skins selected_zip={} selected_psd={} reason=show_default_png_first",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
        ));
        log_server_info(format!(
            "trigger=control_command action=change_character stage=defer_mouth_flap_skins selected_zip={} selected_psd={} reason=lazy_generate_on_timeline",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
        ));
        self.run_timed_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "persist_runtime_state",
            |app| {
                persist_requested_character_change(&app.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to persist requested mascot character to {}",
                            app.runtime_state_path.display()
                        )
                    },
                )
            },
        )?;
        let persisted = self.run_timed_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "verify_runtime_state",
            |app| {
                verify_persisted_character_change(&app.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to verify requested mascot character in {}",
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

    fn run_timed_change_character_stage<T>(
        &mut self,
        previous_png_path: &Path,
        png_path: &Path,
        stage: &'static str,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let started_at = Instant::now();
        let result =
            run_change_character_stage(previous_png_path, png_path, stage, || operation(self));
        self.record_performance_stage(stage, elapsed_ms_since(started_at));
        result
    }
}

fn apply_resolved_character(config: &mut MascotConfig, resolved: &ResolvedCharacterSkin) {
    config.png_path = resolved.png_path.clone();
    config.zip_path = resolved.zip_path.clone();
    config.psd_path_in_zip = resolved.psd_path_in_zip.clone();
    config.display_diff_path = resolved.display_diff_path.clone();
}

fn config_matches_resolved_character(
    config: &MascotConfig,
    resolved: &ResolvedCharacterSkin,
) -> bool {
    config.png_path == resolved.png_path
        && config.zip_path == resolved.zip_path
        && config.psd_path_in_zip == resolved.psd_path_in_zip
        && config.display_diff_path == resolved.display_diff_path
}
