use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::log_server_info;
use mascot_render_core::{LayerVisibilityOverride, MascotEnsembleMode};
use mascot_render_protocol::VptEnsembleRequest;

use super::super::character::{resolve_character_skin, ResolvedCharacterSkin};
use super::super::MascotApp;
use crate::app_support::path_modified_at;
use crate::ensemble::{
    load_vpt_ensemble_entries, save_vpt_ensemble, vpt_ensemble_path, EnsembleEntry,
};

impl MascotApp {
    pub(super) fn set_vpt_ensemble(
        &mut self,
        ctx: &egui::Context,
        request: &VptEnsembleRequest,
    ) -> Result<()> {
        let entries = self.resolve_vpt_ensemble_entries(request)?;
        save_vpt_ensemble(&entries).context("failed to save vpt ensemble entries")?;
        if self.config.ensemble_mode == MascotEnsembleMode::Vpt {
            self.reload_vpt_ensemble_scene(ctx)?;
        } else {
            self.apply_ensemble_mode(ctx, MascotEnsembleMode::Vpt, "control_command")?;
        }
        log_server_info(format!(
            "trigger=control_command action=set_vpt_ensemble result=applied character_count={} entry_count={}",
            request.character_names.len(),
            entries.len()
        ));
        Ok(())
    }

    pub(super) fn set_vpt_ensemble_members(
        &mut self,
        ctx: &egui::Context,
        request: &VptEnsembleRequest,
    ) -> Result<()> {
        let entries = self.resolve_vpt_ensemble_entries(request)?;
        save_vpt_ensemble(&entries).context("failed to save vpt ensemble entries")?;
        let display_reloaded = self.config.ensemble_mode == MascotEnsembleMode::Vpt;
        if display_reloaded {
            self.reload_vpt_ensemble_scene(ctx)?;
        }
        log_server_info(format!(
            "trigger=control_command action=set_vpt_ensemble_members result=applied character_count={} entry_count={} display_reloaded={display_reloaded}",
            request.character_names.len(),
            entries.len()
        ));
        Ok(())
    }

    fn resolve_vpt_ensemble_entries(
        &mut self,
        request: &VptEnsembleRequest,
    ) -> Result<Vec<EnsembleEntry>> {
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

    fn reload_vpt_ensemble_scene(&mut self, ctx: &egui::Context) -> Result<()> {
        let previous_layout = self.window_layout;
        self.ensemble_scene = self.load_active_ensemble_scene(ctx)?;
        self.ensemble_modified_at = path_modified_at(&vpt_ensemble_path());
        let diagnostics = self.refresh_window_layout(ctx, previous_layout);
        self.log_refresh_window_layout("control_command", diagnostics);
        ctx.request_repaint();
        Ok(())
    }
}

fn vpt_entry_from_resolved(
    resolved: &ResolvedCharacterSkin,
    saved_entries: &[EnsembleEntry],
) -> EnsembleEntry {
    let saved = saved_entries.iter().find(|entry| {
        entry.zip_path == resolved.zip_path && entry.psd_path_in_zip == resolved.psd_path_in_zip
    });
    EnsembleEntry {
        character_name: Some(resolved.character_name.clone()),
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
