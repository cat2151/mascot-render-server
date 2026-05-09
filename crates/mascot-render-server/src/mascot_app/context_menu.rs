use eframe::egui;
use mascot_render_control::{log_server_error, log_server_info};
use mascot_render_protocol::PlacementMode;

use super::context_menu_shortcut::{
    placement_context_menu_action_for_input, PlacementContextMenuAction,
};
use super::{persistence::persist_favorite_ensemble_enabled, MascotApp};

impl MascotApp {
    pub(super) fn show_placement_context_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let status = self.placement_status_for_snapshot(ctx);
        let mut handled_action = false;
        let mut favorite_ensemble_enabled = self.config.favorite_ensemble_enabled;
        if ui
            .checkbox(&mut favorite_ensemble_enabled, "F: アンサンブル表示")
            .changed()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::ToggleFavoriteEnsemble,
                status.mode,
            );
        }

        ui.separator();

        if ui
            .selectable_label(
                status.mode == PlacementMode::PerPsd,
                "P: mode1: PSDごとに拡大率と座標を保持する",
            )
            .clicked()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::SetPlacementMode(PlacementMode::PerPsd),
                status.mode,
            );
        }
        if ui
            .selectable_label(
                status.mode == PlacementMode::SharedVisualSize,
                "S: mode2: default: 見た目のheightを全PSDで同一にし、右下アンカーを自動判別する",
            )
            .clicked()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::SetPlacementMode(PlacementMode::SharedVisualSize),
                status.mode,
            );
        }
        if ui.button("Q: アプリをquit").clicked() {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::Quit,
                status.mode,
            );
        }

        if !handled_action {
            if let Some(action) = ui.input(placement_context_menu_action_for_input) {
                self.apply_placement_context_menu_action(ui, ctx, action, status.mode);
            }
        }
    }

    fn apply_placement_context_menu_action(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        action: PlacementContextMenuAction,
        current_mode: PlacementMode,
    ) -> bool {
        match action {
            PlacementContextMenuAction::ToggleFavoriteEnsemble => {
                self.request_favorite_ensemble_toggle(ctx, !self.config.favorite_ensemble_enabled);
                ui.close();
                true
            }
            PlacementContextMenuAction::SetPlacementMode(mode) => {
                if current_mode == mode {
                    return false;
                }
                self.set_placement_mode(ctx, mode);
                ui.close();
                true
            }
            PlacementContextMenuAction::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close();
                true
            }
        }
    }

    fn request_favorite_ensemble_toggle(&mut self, ctx: &egui::Context, enabled: bool) {
        if enabled == self.config.favorite_ensemble_enabled {
            return;
        }

        match persist_favorite_ensemble_enabled(&self.config_path, enabled) {
            Ok(()) => {
                log_server_info(format!(
                    "trigger=context_menu action=toggle_favorite_ensemble result=saved requested_enabled={enabled} config_path={}",
                    self.config_path.display()
                ));
                ctx.request_repaint();
            }
            Err(error) => {
                let message = format!(
                    "trigger=context_menu action=toggle_favorite_ensemble result=failed requested_enabled={enabled} config_path={} error={error:#}",
                    self.config_path.display()
                );
                self.record_status_error(message.clone());
                log_server_error(message);
            }
        }
    }
}
