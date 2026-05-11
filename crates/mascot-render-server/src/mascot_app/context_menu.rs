use eframe::egui;
use mascot_render_control::{log_server_error, log_server_info};
use mascot_render_core::MascotEnsembleMode;
use mascot_render_protocol::PlacementMode;

use super::context_menu_shortcut::{
    placement_context_menu_action_for_input, PlacementContextMenuAction,
};
use super::MascotApp;

impl MascotApp {
    pub(super) fn show_placement_context_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let status = self.placement_status_for_snapshot(ctx);
        let mut handled_action = false;
        if ui
            .selectable_label(
                self.config.ensemble_mode == MascotEnsembleMode::SingleCharacter,
                "1: 通常1char表示",
            )
            .clicked()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::SingleCharacter),
                status.mode,
            );
        }
        if ui
            .selectable_label(
                self.config.ensemble_mode == MascotEnsembleMode::Favorite,
                "F: favoriteアンサンブル表示",
            )
            .clicked()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::Favorite),
                status.mode,
            );
        }
        if ui
            .selectable_label(
                self.config.ensemble_mode == MascotEnsembleMode::Vpt,
                "V: vptアンサンブル表示",
            )
            .clicked()
        {
            handled_action = self.apply_placement_context_menu_action(
                ui,
                ctx,
                PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::Vpt),
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
            PlacementContextMenuAction::SetEnsembleMode(mode) => {
                self.request_ensemble_mode(ui, ctx, mode)
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

    fn request_ensemble_mode(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        mode: MascotEnsembleMode,
    ) -> bool {
        if mode == self.config.ensemble_mode {
            return false;
        }

        match self.apply_ensemble_mode(ctx, mode, "context_menu") {
            Ok(()) => {
                log_server_info(format!(
                    "trigger=context_menu action=set_ensemble_mode result=saved requested_mode={mode:?} config_path={}",
                    self.config_path.display()
                ));
                ui.close();
                ctx.request_repaint();
                true
            }
            Err(error) => {
                let message = format!(
                    "trigger=context_menu action=set_ensemble_mode result=failed requested_mode={mode:?} config_path={} error={error:#}",
                    self.config_path.display()
                );
                self.record_status_error(message.clone());
                log_server_error(message);
                false
            }
        }
    }
}
