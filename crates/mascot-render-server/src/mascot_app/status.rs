use std::path::PathBuf;

use eframe::egui;
use mascot_render_control::{
    log_server_error, log_server_performance_error, log_server_performance_info,
    MascotControlCommand,
};
use mascot_render_protocol::{
    now_unix_ms, MotionTimelineKind, ServerCommandStage, ServerLifecyclePhase, ServerMotionStatus,
    ServerWindowStatus, ServerWorkStatus,
};
use mascot_render_server::window_history::current_viewport_info;

use super::character::configured_character_name_for_status;
use super::MascotApp;

pub(crate) struct ServerWorkGuard {
    status_store: mascot_render_protocol::ServerStatusStore,
    current: ServerWorkStatus,
    previous: Option<ServerWorkStatus>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPerformanceTrace {
    action: &'static str,
    requested_at_unix_ms: u64,
    applying_at_unix_ms: u64,
    applied_at_unix_ms: Option<u64>,
    command_summary: String,
    previous_displayed_png_path: PathBuf,
    stage_durations: Vec<PerformanceStageDuration>,
}

#[derive(Debug, Clone)]
struct PerformanceStageDuration {
    name: &'static str,
    elapsed_ms: u64,
}

impl MascotApp {
    pub(super) fn refresh_status_snapshot(
        &mut self,
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
        let performance_traces = std::mem::take(&mut self.pending_performance_traces);
        let mut previous_displayed_png_path = PathBuf::new();

        if let Err(error) = self.status_store.update(|snapshot| {
            previous_displayed_png_path = snapshot.displayed_png_path.clone();
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
            snapshot.displayed_png_path = displayed_png_path.clone();
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
        }) {
            self.pending_performance_traces = performance_traces;
            log_server_error(format!("failed to update mascot server status: {error:#}"));
            return;
        }

        for trace in performance_traces {
            log_server_performance_info(trace.completed_message(
                heartbeat_at_unix_ms,
                &displayed_png_path,
                &self.config.png_path,
                &previous_displayed_png_path,
            ));
        }
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

    pub(super) fn record_command_applying(&mut self, command: &MascotControlCommand) {
        let applying_at_unix_ms = now_unix_ms();
        let status =
            command
                .status()
                .with_stage(ServerCommandStage::Applying, applying_at_unix_ms, None);
        let mut previous_displayed_png_path = None;
        if let Err(error) = self.status_store.update(|snapshot| {
            previous_displayed_png_path = Some(snapshot.displayed_png_path.clone());
            snapshot.current_command = Some(status);
            snapshot.last_error = None;
        }) {
            log_server_error(format!("failed to update mascot server status: {error:#}"));
            return;
        }
        if let Some(trace) = previous_displayed_png_path.and_then(|path| {
            PendingPerformanceTrace::from_command(command, path, applying_at_unix_ms)
        }) {
            self.pending_performance_traces.push(trace);
        }
    }

    pub(super) fn record_command_applied(&mut self, command: &MascotControlCommand) {
        let applied_at_unix_ms = now_unix_ms();
        let status =
            command
                .status()
                .with_stage(ServerCommandStage::Applied, applied_at_unix_ms, None);
        if let Err(error) = self.status_store.update(|snapshot| {
            snapshot.current_command = None;
            snapshot.last_completed_command = Some(status);
            snapshot.last_error = None;
        }) {
            log_server_error(format!("failed to update mascot server status: {error:#}"));
            return;
        }
        if let Some(trace) = self.active_performance_trace_mut(command) {
            trace.applied_at_unix_ms = Some(applied_at_unix_ms);
        }
    }

    pub(super) fn record_command_failed(
        &mut self,
        command: &MascotControlCommand,
        message: String,
    ) {
        let failed_at_unix_ms = now_unix_ms();
        let failure_message = message.clone();
        let status = command.status().with_stage(
            ServerCommandStage::Failed,
            failed_at_unix_ms,
            Some(message.clone()),
        );
        self.update_status_store(|snapshot| {
            snapshot.current_command = None;
            snapshot.last_failed_command = Some(status);
            snapshot.last_error = Some(message);
        });
        if let Some(trace) = self.remove_active_performance_trace(command) {
            log_server_performance_error(trace.failed_message(failed_at_unix_ms, &failure_message));
        } else if let Some(message) = PendingPerformanceTrace::failed_message_from_command(
            command,
            failed_at_unix_ms,
            &failure_message,
        ) {
            log_server_performance_error(message);
        }
    }

    pub(super) fn record_performance_stage(&mut self, stage: &'static str, elapsed_ms: u64) {
        if let Some(trace) = self
            .pending_performance_traces
            .iter_mut()
            .rev()
            .find(|trace| trace.applied_at_unix_ms.is_none())
        {
            trace.stage_durations.push(PerformanceStageDuration {
                name: stage,
                elapsed_ms,
            });
        }
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

    fn active_performance_trace_mut(
        &mut self,
        command: &MascotControlCommand,
    ) -> Option<&mut PendingPerformanceTrace> {
        self.pending_performance_traces
            .iter_mut()
            .rev()
            .find(|trace| trace.matches_command(command) && trace.applied_at_unix_ms.is_none())
    }

    fn remove_active_performance_trace(
        &mut self,
        command: &MascotControlCommand,
    ) -> Option<PendingPerformanceTrace> {
        let index = self.pending_performance_traces.iter().rposition(|trace| {
            trace.matches_command(command) && trace.applied_at_unix_ms.is_none()
        })?;
        Some(self.pending_performance_traces.remove(index))
    }
}

impl PendingPerformanceTrace {
    fn from_command(
        command: &MascotControlCommand,
        previous_displayed_png_path: PathBuf,
        applying_at_unix_ms: u64,
    ) -> Option<Self> {
        let action = performance_action(command)?;
        Some(Self {
            action,
            requested_at_unix_ms: command.status().requested_at_unix_ms,
            applying_at_unix_ms,
            applied_at_unix_ms: None,
            command_summary: command.status().summary.clone(),
            previous_displayed_png_path,
            stage_durations: Vec::new(),
        })
    }

    fn completed_message(
        &self,
        completed_at_unix_ms: u64,
        displayed_png_path: &std::path::Path,
        configured_png_path: &std::path::Path,
        previous_displayed_png_path: &std::path::Path,
    ) -> String {
        let elapsed_ms = completed_at_unix_ms.saturating_sub(self.requested_at_unix_ms);
        let applied_at_unix_ms = self.applied_at_unix_ms.unwrap_or(completed_at_unix_ms);
        let queue_ms = self
            .applying_at_unix_ms
            .saturating_sub(self.requested_at_unix_ms);
        let apply_ms = applied_at_unix_ms.saturating_sub(self.applying_at_unix_ms);
        let settle_ms = completed_at_unix_ms.saturating_sub(applied_at_unix_ms);
        let texture_changed = self.previous_displayed_png_path != displayed_png_path;
        format!(
            "event=post_to_status_settled action={} result=completed elapsed_ms={} queue_ms={} apply_ms={} settle_ms={} requested_at_unix_ms={} applying_at_unix_ms={} applied_at_unix_ms={} completed_at_unix_ms={} status_settled=true texture_changed={} stage_ms={} previous_displayed_png_path={} displayed_png_path={} configured_png_path={} command_summary={}",
            self.action,
            elapsed_ms,
            queue_ms,
            apply_ms,
            settle_ms,
            self.requested_at_unix_ms,
            self.applying_at_unix_ms,
            applied_at_unix_ms,
            completed_at_unix_ms,
            texture_changed,
            self.stage_summary(),
            previous_displayed_png_path.display(),
            displayed_png_path.display(),
            configured_png_path.display(),
            self.command_summary
        )
    }

    fn failed_message(&self, failed_at_unix_ms: u64, error: &str) -> String {
        let elapsed_ms = failed_at_unix_ms.saturating_sub(self.requested_at_unix_ms);
        let queue_ms = self
            .applying_at_unix_ms
            .saturating_sub(self.requested_at_unix_ms);
        let apply_ms = failed_at_unix_ms.saturating_sub(self.applying_at_unix_ms);
        format!(
            "event=post_to_status_settled action={} result=failed elapsed_ms={} queue_ms={} apply_ms={} settle_ms=0 requested_at_unix_ms={} applying_at_unix_ms={} failed_at_unix_ms={} status_settled=false stage_ms={} command_summary={} error={error}",
            self.action,
            elapsed_ms,
            queue_ms,
            apply_ms,
            self.requested_at_unix_ms,
            self.applying_at_unix_ms,
            failed_at_unix_ms,
            self.stage_summary(),
            self.command_summary
        )
    }

    fn failed_message_from_command(
        command: &MascotControlCommand,
        failed_at_unix_ms: u64,
        error: &str,
    ) -> Option<String> {
        let action = performance_action(command)?;
        let requested_at_unix_ms = command.status().requested_at_unix_ms;
        let elapsed_ms = failed_at_unix_ms.saturating_sub(requested_at_unix_ms);
        Some(format!(
            "event=post_to_status_settled action={action} result=failed elapsed_ms={elapsed_ms} queue_ms=0 apply_ms={elapsed_ms} settle_ms=0 requested_at_unix_ms={requested_at_unix_ms} applying_at_unix_ms={requested_at_unix_ms} failed_at_unix_ms={failed_at_unix_ms} status_settled=false stage_ms=none command_summary={} error={error}",
            command.status().summary
        ))
    }

    fn matches_command(&self, command: &MascotControlCommand) -> bool {
        self.requested_at_unix_ms == command.status().requested_at_unix_ms
            && performance_action(command) == Some(self.action)
    }

    fn stage_summary(&self) -> String {
        if self.stage_durations.is_empty() {
            return "none".to_string();
        }

        self.stage_durations
            .iter()
            .map(|duration| format!("{}:{}ms", duration.name, duration.elapsed_ms))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn performance_action(command: &MascotControlCommand) -> Option<&'static str> {
    match command {
        MascotControlCommand::ChangeCharacter { .. } => Some("change_character"),
        MascotControlCommand::PlayTimeline { request, .. }
            if request
                .steps
                .iter()
                .any(|step| step.kind == MotionTimelineKind::MouthFlap) =>
        {
            Some("timeline_mouth_flap")
        }
        MascotControlCommand::Show { .. }
        | MascotControlCommand::Hide { .. }
        | MascotControlCommand::PlayTimeline { .. } => None,
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

#[cfg(test)]
impl PendingPerformanceTrace {
    pub(crate) fn from_command_for_test(
        command: &MascotControlCommand,
        previous_displayed_png_path: PathBuf,
    ) -> Option<Self> {
        Self::from_command(
            command,
            previous_displayed_png_path,
            command.status().requested_at_unix_ms + 7,
        )
    }

    pub(crate) fn mark_applied_for_test(&mut self, applied_at_unix_ms: u64) {
        self.applied_at_unix_ms = Some(applied_at_unix_ms);
    }

    pub(crate) fn record_stage_for_test(&mut self, stage: &'static str, elapsed_ms: u64) {
        self.stage_durations.push(PerformanceStageDuration {
            name: stage,
            elapsed_ms,
        });
    }

    pub(crate) fn completed_message_for_test(
        &self,
        completed_at_unix_ms: u64,
        displayed_png_path: &std::path::Path,
        configured_png_path: &std::path::Path,
        previous_displayed_png_path: &std::path::Path,
    ) -> String {
        self.completed_message(
            completed_at_unix_ms,
            displayed_png_path,
            configured_png_path,
            previous_displayed_png_path,
        )
    }
}
