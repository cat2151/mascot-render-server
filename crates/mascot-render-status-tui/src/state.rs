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
    performance_log_lines: Vec<String>,
    performance_log_error: Option<String>,
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

    pub(crate) fn record_performance_log_snapshot(&mut self, lines: Vec<String>) {
        self.performance_log_lines = lines;
        self.performance_log_error = None;
    }

    pub(crate) fn record_performance_log_error(&mut self, error: String) {
        self.performance_log_error = Some(error);
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
            TestPostStatus::Running(label) => {
                format!("running: {}", compact_test_post_label(label))
            }
            TestPostStatus::Succeeded { label, elapsed_ms } => {
                format!(
                    "ok ({}): {}",
                    format_duration_ms(*elapsed_ms),
                    compact_test_post_label(label)
                )
            }
            TestPostStatus::Failed { label, error } => {
                format!(
                    "failed: {error} | action={}",
                    compact_test_post_label(label)
                )
            }
        }
    }

    pub(crate) fn performance_log_lines(&self) -> Vec<String> {
        if let Some(error) = self.performance_log_error.as_ref() {
            return vec![format!("error: {error}")];
        }
        if self.performance_log_lines.is_empty() {
            vec!["-".to_string()]
        } else {
            self.performance_log_lines.clone()
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

fn compact_test_post_label(label: &str) -> String {
    if label.starts_with("change-character random cached PSD:") {
        let name = key_value(label, "generated_character_name").unwrap_or("-");
        let png = key_value(label, "random_png")
            .map(compact_path_tail)
            .unwrap_or_else(|| "-".to_string());
        return format!("random cached PSD | name={name} | png={png}");
    }

    match label {
        "change-character configured_character_name" => "change-character configured".to_string(),
        "timeline mouth-flap" => "timeline mouth-flap".to_string(),
        "timeline shake" => "timeline shake".to_string(),
        "show" | "hide" => label.to_string(),
        _ => shorten_middle(label, 80),
    }
}

fn key_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    text.split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}

fn compact_path_tail(path: &str) -> String {
    let tail = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|tail| !tail.is_empty())
        .unwrap_or(path);
    shorten_middle(tail, 36)
}

fn shorten_middle(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let head_len = (max_chars - 3) / 2;
    let tail_len = max_chars - 3 - head_len;
    let head = text.chars().take(head_len).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}...{tail}")
}
