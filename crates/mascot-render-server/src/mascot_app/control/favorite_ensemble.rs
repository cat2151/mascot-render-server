use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::log_server_info;
use mascot_render_core::{LayerVisibilityOverride, MascotEnsembleMode};
use mascot_render_protocol::{ServerEnsembleMode, VptEnsembleRequest};

use super::super::character::{resolve_character_skin, ResolvedCharacterSkin};
use super::super::persistence::persist_ensemble_mode;
use super::super::MascotApp;
use crate::favorite_ensemble::{
    load_vpt_ensemble_entries, save_vpt_ensemble, FavoriteEnsembleEntry,
};

impl MascotApp {
    pub(super) fn set_ensemble_mode_from_control(
        &mut self,
        ctx: &egui::Context,
        mode: ServerEnsembleMode,
    ) -> Result<()> {
        let mode = core_ensemble_mode(mode);
        self.apply_ensemble_mode(ctx, mode, "control_command")
    }

    pub(super) fn set_vpt_ensemble(
        &mut self,
        ctx: &egui::Context,
        request: &VptEnsembleRequest,
    ) -> Result<()> {
        let entries = self.resolve_vpt_ensemble_entries(request)?;
        save_vpt_ensemble(&entries).context("failed to save vpt ensemble entries")?;
        self.apply_ensemble_mode(ctx, MascotEnsembleMode::Vpt, "control_command")?;
        log_server_info(format!(
            "trigger=control_command action=set_vpt_ensemble result=applied character_count={} entry_count={}",
            request.character_names.len(),
            entries.len()
        ));
        Ok(())
    }

    pub(in crate::mascot_app) fn apply_ensemble_mode(
        &mut self,
        ctx: &egui::Context,
        mode: MascotEnsembleMode,
        trigger: &str,
    ) -> Result<()> {
        let previous_mode = self.config.ensemble_mode;
        if previous_mode == mode {
            return Ok(());
        }

        log_server_info(format!(
            "trigger={trigger} action=set_ensemble_mode previous_mode={previous_mode:?} requested_mode={mode:?} 適用を開始しました: config_path={}",
            self.config_path.display()
        ));
        persist_ensemble_mode(&self.config_path, mode).with_context(|| {
            format!(
                "failed to persist ensemble mode {mode:?} to {}",
                self.config_path.display()
            )
        })?;
        self.config_modified_at = None;
        self.reload_config_if_needed(ctx).with_context(|| {
            format!(
                "failed to apply ensemble mode {mode:?} from {}",
                self.config_path.display()
            )
        })?;

        if self.config.ensemble_mode != mode {
            bail!(
                "ensemble_mode remained {:?} after setting {:?} via {}",
                self.config.ensemble_mode,
                mode,
                self.config_path.display()
            );
        }

        log_server_info(format!(
            "trigger={trigger} action=set_ensemble_mode result=applied previous_mode={previous_mode:?} mode={mode:?} config_path={}",
            self.config_path.display()
        ));
        Ok(())
    }

    fn resolve_vpt_ensemble_entries(
        &mut self,
        request: &VptEnsembleRequest,
    ) -> Result<Vec<FavoriteEnsembleEntry>> {
        let saved_entries = load_vpt_ensemble_entries().unwrap_or_else(|error| {
            log_server_info(format!(
                "trigger=control_command action=set_vpt_ensemble stage=load_saved_positions result=ignored error={error:#}"
            ));
            Vec::new()
        });
        let mut entries = Vec::with_capacity(request.character_names.len());
        for character_name in &request.character_names {
            let resolved =
                resolve_character_skin(&self.core, character_name).with_context(|| {
                    format!(
                        "failed to resolve vpt ensemble character: character_name={character_name}"
                    )
                })?;
            log_server_info(format!(
                "trigger=control_command action=set_vpt_ensemble stage=resolve character_name={} candidate_count={} selected_zip={} selected_psd={} selected_png={}",
                resolved.character_name,
                resolved.candidate_count,
                resolved.zip_path.display(),
                resolved.psd_path_in_zip.display(),
                resolved.png_path.display()
            ));
            entries.push(vpt_entry_from_resolved(&resolved, &saved_entries));
        }
        Ok(entries)
    }
}

fn core_ensemble_mode(mode: ServerEnsembleMode) -> MascotEnsembleMode {
    match mode {
        ServerEnsembleMode::SingleCharacter => MascotEnsembleMode::SingleCharacter,
        ServerEnsembleMode::Favorite => MascotEnsembleMode::Favorite,
        ServerEnsembleMode::Vpt => MascotEnsembleMode::Vpt,
    }
}

fn vpt_entry_from_resolved(
    resolved: &ResolvedCharacterSkin,
    saved_entries: &[FavoriteEnsembleEntry],
) -> FavoriteEnsembleEntry {
    let saved = saved_entries.iter().find(|entry| {
        entry.zip_path == resolved.zip_path && entry.psd_path_in_zip == resolved.psd_path_in_zip
    });
    FavoriteEnsembleEntry {
        zip_path: resolved.zip_path.clone(),
        psd_path_in_zip: resolved.psd_path_in_zip.clone(),
        psd_file_name: resolved
            .psd_path_in_zip
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| resolved.psd_path_in_zip.display().to_string()),
        visibility_overrides: saved
            .map(|entry| entry.visibility_overrides.clone())
            .unwrap_or_else(Vec::<LayerVisibilityOverride>::new),
        mascot_scale: saved.and_then(|entry| entry.mascot_scale),
        favorite_ensemble_position: saved.and_then(|entry| entry.favorite_ensemble_position),
    }
}
