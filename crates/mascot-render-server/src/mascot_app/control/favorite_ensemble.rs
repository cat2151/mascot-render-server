use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::log_server_info;

use super::super::persistence::persist_favorite_ensemble_enabled;
use super::super::MascotApp;

impl MascotApp {
    pub(super) fn disable_favorite_ensemble(&mut self, ctx: &egui::Context) -> Result<()> {
        let was_enabled = self.config.favorite_ensemble_enabled;
        log_server_info(format!(
            "trigger=control_command action=disable_favorite_ensemble favorite_ensemble_enabled={was_enabled} 無効化を開始しました: config_path={}",
            self.config_path.display()
        ));

        persist_favorite_ensemble_enabled(&self.config_path, false).with_context(|| {
            format!(
                "failed to persist favorite ensemble disabled state to {}",
                self.config_path.display()
            )
        })?;
        self.config_modified_at = None;
        self.reload_config_if_needed(ctx).with_context(|| {
            format!(
                "failed to apply favorite ensemble disabled state from {}",
                self.config_path.display()
            )
        })?;

        if self.config.favorite_ensemble_enabled {
            bail!(
                "favorite_ensemble_enabled remained true after disabling via {}",
                self.config_path.display()
            );
        }

        log_server_info(format!(
            "trigger=control_command action=disable_favorite_ensemble result=applied previous_favorite_ensemble_enabled={was_enabled} config_path={}",
            self.config_path.display()
        ));
        Ok(())
    }
}
