use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::log_server_info;
use mascot_render_core::MascotEnsembleMode;
use mascot_render_protocol::ServerEnsembleMode;

use super::super::persistence::persist_ensemble_mode;
use super::super::MascotApp;

impl MascotApp {
    pub(super) fn set_ensemble_mode_from_control(
        &mut self,
        ctx: &egui::Context,
        mode: ServerEnsembleMode,
    ) -> Result<()> {
        let mode = core_ensemble_mode(mode);
        self.apply_ensemble_mode(ctx, mode, "control_command")
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
}

fn core_ensemble_mode(mode: ServerEnsembleMode) -> MascotEnsembleMode {
    match mode {
        ServerEnsembleMode::SingleCharacter => MascotEnsembleMode::SingleCharacter,
        ServerEnsembleMode::Favorite => MascotEnsembleMode::Favorite,
        ServerEnsembleMode::Vpt => MascotEnsembleMode::Vpt,
    }
}
