use std::path::PathBuf;

use eframe::egui;
use mascot_render_control::{log_server_error, MascotControlCommand};
use mascot_render_protocol::{
    now_unix_ms, ServerCommandStage, ServerLifecyclePhase, ServerMotionStatus, ServerWindowStatus,
    ServerWorkStatus,
};
use mascot_render_server::window_history::current_viewport_info;

use super::character::configured_character_name_for_status;
use super::MascotApp;

pub(crate) struct ServerWorkGuard {
    status_store: mascot_render_protocol::ServerStatusStore,
    current: ServerWorkStatus,
    previous: Option<ServerWorkStatus>,
}

impl MascotApp {
    pub(super) fn refresh_status_snapshot(
        &self,
        ctx: &egui::Context,
        displayed_png_path: PathBuf,
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
            snapshot.configured_character_name = configured_character_name_for_status(
                &self.config.zip_path,
                &self.config.psd_path_in_zip,
            );
            snapshot.configured_png_path = self.config.png_path.clone();
            snapshot.configured_zip_path = self.config.zip_path.clone();
            snapshot.configured_psd_path_in_zip = self.config.psd_path_in_zip.clone();
            snapshot.displayed_png_path = displayed_png_path;
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

    pub(super) fn start_current_work(
        &self,
        kind: &'static str,
        stage: &'static str,
        summary: impl Into<String>,
    ) -> ServerWorkGuard {
        ServerWorkGuard::start(self.status_store.clone(), kind, stage, summary.into())
    }

    fn update_status_store(
        &self,
        update: impl FnOnce(&mut mascot_render_protocol::ServerStatusSnapshot),
    ) {
        update_status_store(&self.status_store, update);
    }
}

impl ServerWorkGuard {
    fn start(
        status_store: mascot_render_protocol::ServerStatusStore,
        kind: &'static str,
        stage: &'static str,
        summary: String,
    ) -> Self {
        let current = ServerWorkStatus::started(kind, stage, summary);
        let mut previous = None;
        update_status_store(&status_store, |snapshot| {
            previous = snapshot.current_work.clone();
            snapshot.current_work = Some(current.clone());
        });
        Self {
            status_store,
            current,
            previous,
        }
    }

    pub(crate) fn update_stage(&mut self, stage: &'static str, summary: impl Into<String>) {
        let current = self.current.with_stage(stage, summary.into());
        update_status_store(&self.status_store, |snapshot| {
            if current_work_matches(snapshot.current_work.as_ref(), &self.current) {
                snapshot.current_work = Some(current.clone());
            }
        });
        self.current = current;
    }
}

impl Drop for ServerWorkGuard {
    fn drop(&mut self) {
        update_status_store(&self.status_store, |snapshot| {
            if current_work_matches(snapshot.current_work.as_ref(), &self.current) {
                snapshot.current_work = self.previous.clone();
            }
        });
    }
}

fn current_work_matches(current: Option<&ServerWorkStatus>, expected: &ServerWorkStatus) -> bool {
    current.is_some_and(|current| {
        current.kind == expected.kind
            && current.started_at_unix_ms == expected.started_at_unix_ms
            && current.stage == expected.stage
    })
}

fn update_status_store(
    status_store: &mascot_render_protocol::ServerStatusStore,
    update: impl FnOnce(&mut mascot_render_protocol::ServerStatusSnapshot),
) {
    if let Err(error) = status_store.update(update) {
        log_server_error(format!("failed to update mascot server status: {error:#}"));
    }
}

#[cfg(test)]
impl ServerWorkGuard {
    pub(crate) fn start_for_test(
        status_store: mascot_render_protocol::ServerStatusStore,
        kind: &'static str,
        stage: &'static str,
        summary: impl Into<String>,
    ) -> Self {
        Self::start(status_store, kind, stage, summary.into())
    }
}
