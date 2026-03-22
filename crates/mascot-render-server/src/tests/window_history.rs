use std::fs;
use std::time::Instant;

use eframe::egui::Pos2;
use mascot_render_core::workspace_cache_root;

use crate::window_history::{
    load_window_position, WindowHistoryTracker, WINDOW_HISTORY_SAVE_DEBOUNCE,
};

#[test]
fn window_history_round_trips_saved_position() {
    let path = workspace_cache_root().join("test-window-history/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history"));

    let mut tracker = WindowHistoryTracker::new(path.clone(), None);
    let now = Instant::now();
    tracker
        .observe(Pos2::new(120.0, 48.0), now)
        .expect("should observe position");
    tracker.flush().expect("should save position");

    let loaded = load_window_position(&path).expect("should read saved history");
    assert_eq!(loaded, Some(Pos2::new(120.0, 48.0)));
}

#[test]
fn invalid_window_history_is_reported() {
    let path = workspace_cache_root().join("test-window-history-invalid/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history-invalid"));
    fs::create_dir_all(workspace_cache_root().join("test-window-history-invalid"))
        .expect("should create temp directory");
    fs::write(&path, "{ invalid json").expect("should seed invalid history");

    let error = load_window_position(&path).expect_err("invalid history should fail");
    assert!(error.to_string().contains("failed to parse window history"));
}

#[test]
fn tracker_saves_after_position_stabilizes() {
    let path = workspace_cache_root().join("test-window-history-debounce/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history-debounce"));

    let mut tracker = WindowHistoryTracker::new(path.clone(), None);
    let now = Instant::now();
    tracker
        .observe(Pos2::new(20.0, 30.0), now)
        .expect("should observe initial position");
    assert!(
        !path.exists(),
        "history should not be written before the debounce elapses"
    );

    tracker
        .observe(Pos2::new(20.0, 30.0), now + WINDOW_HISTORY_SAVE_DEBOUNCE)
        .expect("should observe stabilized position");

    let loaded = load_window_position(&path).expect("should read saved history");
    assert_eq!(loaded, Some(Pos2::new(20.0, 30.0)));
}

