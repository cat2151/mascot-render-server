use std::time::Instant;

use mascot_render_protocol::ServerStatusSnapshot;

use crate::actions::CachedPsdSource;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ServerStartupStatus {
    #[default]
    Idle,
    Starting,
    Started,
    Failed(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum TestPostStatus {
    #[default]
    Idle,
    Running(String),
    Succeeded {
        label: String,
        elapsed_ms: u64,
    },
    Failed {
        label: String,
        error: String,
    },
}

#[derive(Debug, Default)]
pub(crate) struct StatusTuiState {
    pub(crate) last_snapshot: Option<ServerStatusSnapshot>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_success_at: Option<Instant>,
    pub(crate) poll_in_flight: bool,
    pub(crate) startup_status: ServerStartupStatus,
    pub(crate) test_post_status: TestPostStatus,
    help_visible: bool,
    should_quit: bool,
}

impl StatusTuiState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record_poll_started(&mut self) {
        self.poll_in_flight = true;
    }

    pub(crate) fn record_poll_success(&mut self, snapshot: ServerStatusSnapshot, now: Instant) {
        self.last_snapshot = Some(snapshot);
        self.last_success_at = Some(now);
        self.last_error = None;
        self.poll_in_flight = false;
        if !matches!(self.startup_status, ServerStartupStatus::Idle) {
            self.startup_status = ServerStartupStatus::Started;
        }
    }

    pub(crate) fn record_poll_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.poll_in_flight = false;
    }

    pub(crate) fn record_startup_starting(&mut self) {
        self.startup_status = ServerStartupStatus::Starting;
    }

    pub(crate) fn record_startup_started(&mut self) {
        self.startup_status = ServerStartupStatus::Started;
    }

    pub(crate) fn record_startup_failed(&mut self, error: String) {
        self.startup_status = ServerStartupStatus::Failed(error);
    }

    pub(crate) fn record_test_post_started(&mut self, label: String) {
        self.test_post_status = TestPostStatus::Running(label);
    }

    pub(crate) fn record_test_post_success(&mut self, label: String, elapsed_ms: u64) {
        self.test_post_status = TestPostStatus::Succeeded { label, elapsed_ms };
    }

    pub(crate) fn record_test_post_failed(&mut self, label: String, error: String) {
        self.test_post_status = TestPostStatus::Failed { label, error };
    }

    pub(crate) fn startup_status_summary(&self) -> &'static str {
        match self.startup_status {
            ServerStartupStatus::Idle => "idle",
            ServerStartupStatus::Starting => "starting",
            ServerStartupStatus::Started => "started",
            ServerStartupStatus::Failed(_) => "failed",
        }
    }

    pub(crate) fn startup_error(&self) -> Option<&str> {
        match &self.startup_status {
            ServerStartupStatus::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub(crate) fn test_post_status_label(&self) -> String {
        match &self.test_post_status {
            TestPostStatus::Idle => "idle".to_string(),
            TestPostStatus::Running(label) => format!("{label}: running"),
            TestPostStatus::Succeeded { label, elapsed_ms } => {
                format!("{label}: ok ({})", format_duration_ms(*elapsed_ms))
            }
            TestPostStatus::Failed { label, error } => format!("{label}: failed: {error}"),
        }
    }

    pub(crate) fn poll_status_label(&self) -> &'static str {
        if self.poll_in_flight {
            "polling"
        } else {
            "idle"
        }
    }

    pub(crate) fn configured_character_name(&self) -> Option<String> {
        self.last_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.configured_character_name.clone())
    }

    pub(crate) fn current_psd_source(&self) -> Option<CachedPsdSource> {
        self.last_snapshot.as_ref().map(|snapshot| CachedPsdSource {
            png_path: snapshot.configured_png_path.clone(),
            zip_path: snapshot.configured_zip_path.clone(),
            psd_path_in_zip: snapshot.configured_psd_path_in_zip.clone(),
        })
    }

    pub(crate) fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    pub(crate) fn close_help(&mut self) {
        self.help_visible = false;
    }

    pub(crate) fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.last_success_at.is_some() && self.last_error.is_none()
    }

    pub(crate) fn connection_label(&self) -> &'static str {
        if self.is_connected() {
            "connected"
        } else {
            "disconnected"
        }
    }

    pub(crate) fn last_success_age_ms(&self, now: Instant) -> Option<u64> {
        let last_success_at = self.last_success_at?;
        let age = now.checked_duration_since(last_success_at)?;
        Some(u64::try_from(age.as_millis()).unwrap_or(u64::MAX))
    }

    pub(crate) fn request_quit(&mut self) {
        self.should_quit = true;
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }
}

pub(crate) fn heartbeat_age_ms_at(snapshot: &ServerStatusSnapshot, now_unix_ms: u64) -> u64 {
    now_unix_ms.saturating_sub(snapshot.heartbeat_at_unix_ms)
}

pub(crate) fn format_duration_ms(ms: u64) -> String {
    if ms < 1_000 {
        return format!("{ms}ms");
    }

    if ms < 60_000 {
        let seconds = ms / 1_000;
        let tenths = ms % 1_000 / 100;
        return format!("{seconds}.{tenths}s");
    }

    if ms < 3_600_000 {
        let minutes = ms / 60_000;
        let seconds = ms % 60_000 / 1_000;
        return format!("{minutes}m {seconds}s");
    }

    let hours = ms / 3_600_000;
    let minutes = ms % 3_600_000 / 60_000;
    format!("{hours}h {minutes}m")
}
