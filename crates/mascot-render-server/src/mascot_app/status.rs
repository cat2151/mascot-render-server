use std::path::PathBuf;

use eframe::egui;
use mascot_render_control::{log_server_error, MascotControlCommand};
use mascot_render_protocol::{
    now_unix_ms, ServerCommandStage, ServerLifecyclePhase, ServerMotionStatus, ServerWindowStatus,
};
use mascot_render_server::window_history::current_viewport_info;

use super::MascotApp;

impl MascotApp {
    pub(super) fn refresh_status_snapshot(
        &self,
        ctx: &egui::Context,
        current_png_path: PathBuf,
        blink_closed: bool,
        mouth_flap_open: Option<bool>,
    ) {
        let heartbeat_at_unix_ms = now_unix_ms();
        let window_size = self.window_layout.window_size();
        let anchor_position = current_viewport_info(ctx).map(|viewport_info| {
            let position = viewport_info.inner_origin + self.window_layout.anchor_offset();
            [position.x, position.y]
        });

        self.update_status_store(|snapshot| {
            snapshot.captured_at_unix_ms = heartbeat_at_unix_ms;
            snapshot.heartbeat_at_unix_ms = heartbeat_at_unix_ms;
            snapshot.lifecycle = ServerLifecyclePhase::Running;
            snapshot.current_png_path = current_png_path;
            snapshot.favorite_ensemble_enabled = self.config.favorite_ensemble_enabled;
            snapshot.favorite_ensemble_loaded = self.favorite_ensemble.is_some();
            snapshot.scale = self.scale;
            snapshot.motion = ServerMotionStatus {
                active: self.motion.is_active(),
                blink_closed,
                mouth_flap_open,
            };
            snapshot.window = ServerWindowStatus {
                anchor_position,
                window_size: [window_size.x, window_size.y],
            };
            snapshot.config_path = self.config_path.clone();
            snapshot.runtime_state_path = self.runtime_state_path.clone();
            snapshot.pending_persisted_scale = self.pending_persisted_scale.is_some();
        });
    }

    pub(super) fn record_lifecycle_running(&self) {
        self.update_status_store(|snapshot| {
            snapshot.lifecycle = ServerLifecyclePhase::Running;
        });
    }

    pub(super) fn record_lifecycle_stopping(&self) {
        self.update_status_store(|snapshot| {
            snapshot.lifecycle = ServerLifecyclePhase::Stopping;
        });
    }

    pub(super) fn record_status_error(&self, message: String) {
        self.update_status_store(|snapshot| {
            snapshot.last_error = Some(message);
        });
    }

    pub(super) fn record_command_applying(&self, command: &MascotControlCommand) {
        let status = command
            .status()
            .with_stage(ServerCommandStage::Applying, now_unix_ms(), None);
        self.update_status_store(|snapshot| {
            snapshot.current_command = Some(status);
            snapshot.last_error = None;
        });
    }

    pub(super) fn record_command_applied(&self, command: &MascotControlCommand) {
        let status = command
            .status()
            .with_stage(ServerCommandStage::Applied, now_unix_ms(), None);
        self.update_status_store(|snapshot| {
            snapshot.current_command = None;
            snapshot.last_completed_command = Some(status);
            snapshot.last_error = None;
        });
    }

    pub(super) fn record_command_failed(&self, command: &MascotControlCommand, message: String) {
        let status = command.status().with_stage(
            ServerCommandStage::Failed,
            now_unix_ms(),
            Some(message.clone()),
        );
        self.update_status_store(|snapshot| {
            snapshot.current_command = None;
            snapshot.last_failed_command = Some(status);
            snapshot.last_error = Some(message);
        });
    }

    fn update_status_store(
        &self,
        update: impl FnOnce(&mut mascot_render_protocol::ServerStatusSnapshot),
    ) {
        if let Err(error) = self.status_store.update(update) {
            log_server_error(format!("failed to update mascot server status: {error:#}"));
        }
    }
}
