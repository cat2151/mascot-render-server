use std::path::PathBuf;
use std::time::{Duration, Instant};

use mascot_render_protocol::{
    ServerCommandKind, ServerCommandStage, ServerCommandStatus, ServerLifecyclePhase,
    ServerStatusSnapshot,
};

use crate::state::{format_duration_ms, heartbeat_age_ms_at, StatusTuiState};
use crate::ui::{command_status_text, lifecycle_text, option_bool_text, point_text};

#[test]
fn poll_success_updates_snapshot_and_success_time() {
    let mut state = StatusTuiState::new();
    state.record_poll_error("old connection error".to_string());

    let snapshot = sample_snapshot();
    let now = Instant::now();
    state.record_poll_started();
    state.record_poll_success(snapshot.clone(), now);

    assert_eq!(state.last_snapshot, Some(snapshot));
    assert_eq!(state.last_success_at, Some(now));
    assert_eq!(state.last_error, None);
    assert_eq!(state.poll_status_label(), "idle");
    assert_eq!(state.connection_label(), "connected");
}

#[test]
fn poll_error_keeps_existing_snapshot() {
    let mut state = StatusTuiState::new();
    let snapshot = sample_snapshot();
    state.record_poll_success(snapshot.clone(), Instant::now());

    state.record_poll_started();
    state.record_poll_error("failed to connect".to_string());

    assert_eq!(state.last_snapshot, Some(snapshot));
    assert_eq!(state.last_error.as_deref(), Some("failed to connect"));
    assert_eq!(state.poll_status_label(), "idle");
    assert_eq!(state.connection_label(), "disconnected");
}

#[test]
fn poll_started_marks_polling_without_clearing_existing_snapshot() {
    let mut state = StatusTuiState::new();
    let snapshot = sample_snapshot();
    state.record_poll_success(snapshot.clone(), Instant::now());

    state.record_poll_started();

    assert_eq!(state.last_snapshot, Some(snapshot));
    assert_eq!(state.poll_status_label(), "polling");
}

#[test]
fn startup_status_tracks_background_startup_result() {
    let mut state = StatusTuiState::new();

    state.record_startup_starting();
    assert_eq!(state.startup_status_summary(), "starting");

    state.record_startup_failed("spawn failed".to_string());
    assert_eq!(state.startup_status_summary(), "failed");
    assert_eq!(state.startup_error(), Some("spawn failed"));

    state.record_startup_started();
    assert_eq!(state.startup_status_summary(), "started");
    assert_eq!(state.startup_error(), None);
}

#[test]
fn poll_success_marks_pending_startup_as_started() {
    let mut state = StatusTuiState::new();
    state.record_startup_starting();

    state.record_poll_success(sample_snapshot(), Instant::now());

    assert_eq!(state.startup_status_summary(), "started");
}

#[test]
fn test_post_status_tracks_background_post_result() {
    let mut state = StatusTuiState::new();

    state.record_test_post_started("show".to_string());
    assert_eq!(state.test_post_status_label(), "show: running");

    state.record_test_post_success("show".to_string());
    assert_eq!(state.test_post_status_label(), "show: ok");

    state.record_test_post_failed("hide".to_string(), "connection refused".to_string());
    assert_eq!(
        state.test_post_status_label(),
        "hide: failed: connection refused"
    );
}

#[test]
fn configured_character_name_is_none_until_first_snapshot() {
    let mut state = StatusTuiState::new();
    assert_eq!(state.configured_character_name(), None);

    let mut snapshot = sample_snapshot();
    snapshot.configured_character_name = Some("demo".to_string());
    state.record_poll_success(snapshot, Instant::now());

    assert_eq!(state.configured_character_name(), Some("demo".to_string()));
}

#[test]
fn help_visibility_toggles_and_closes() {
    let mut state = StatusTuiState::new();

    assert!(!state.is_help_visible());
    state.toggle_help();
    assert!(state.is_help_visible());
    state.close_help();
    assert!(!state.is_help_visible());
}

#[test]
fn last_success_age_uses_instant_clock() {
    let mut state = StatusTuiState::new();
    let now = Instant::now();
    state.record_poll_success(sample_snapshot(), now);

    assert_eq!(
        state.last_success_age_ms(now + Duration::from_millis(1_234)),
        Some(1_234)
    );
}

#[test]
fn heartbeat_age_and_duration_are_formatted_for_status_display() {
    let mut snapshot = sample_snapshot();
    snapshot.heartbeat_at_unix_ms = 1_000;

    assert_eq!(heartbeat_age_ms_at(&snapshot, 3_345), 2_345);
    assert_eq!(format_duration_ms(999), "999ms");
    assert_eq!(format_duration_ms(2_345), "2.3s");
    assert_eq!(format_duration_ms(65_000), "1m 5s");
    assert_eq!(format_duration_ms(3_660_000), "1h 1m");
}

#[test]
fn command_status_text_formats_kind_stage_summary_and_error() {
    let command = ServerCommandStatus {
        kind: ServerCommandKind::Timeline,
        stage: ServerCommandStage::Failed,
        summary: "shake".to_string(),
        requested_at_unix_ms: 10,
        updated_at_unix_ms: 20,
        error: Some("motion failed".to_string()),
    };

    let text = command_status_text(Some(&command));

    assert!(text.contains("kind: timeline"), "unexpected text: {text}");
    assert!(text.contains("stage: failed"), "unexpected text: {text}");
    assert!(text.contains("summary: shake"), "unexpected text: {text}");
    assert!(
        text.contains("error: motion failed"),
        "unexpected text: {text}"
    );
}

#[test]
fn none_fields_are_visible_as_placeholders() {
    assert_eq!(command_status_text(None), "-");
    assert_eq!(option_bool_text(None), "-");
    assert_eq!(point_text(None), "-");
}

#[test]
fn lifecycle_text_uses_protocol_names() {
    assert_eq!(lifecycle_text(ServerLifecyclePhase::Starting), "starting");
    assert_eq!(lifecycle_text(ServerLifecyclePhase::Running), "running");
    assert_eq!(lifecycle_text(ServerLifecyclePhase::Stopping), "stopping");
}

fn sample_snapshot() -> ServerStatusSnapshot {
    let mut snapshot = ServerStatusSnapshot::starting(
        PathBuf::from("config/mascot-render-server.toml"),
        PathBuf::from("config/mascot-render-server.runtime.json"),
        PathBuf::from("cache/demo/open.png"),
        PathBuf::from("assets/zip/demo.zip"),
        PathBuf::from("demo/basic.psd"),
    );
    snapshot.lifecycle = ServerLifecyclePhase::Running;
    snapshot
}
