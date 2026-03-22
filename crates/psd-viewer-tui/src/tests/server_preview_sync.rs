use std::path::PathBuf;

use crate::server_preview_sync::ServerPreviewSyncState;

#[test]
fn starts_server_sync_immediately_for_first_target() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    let started = state.request(Some(first.as_path()));

    assert_eq!(started.as_deref(), Some(first.as_path()));
    assert_eq!(state.active_png_path_for_test(), Some(first.as_path()));
    assert!(state.is_busy());
}

#[test]
fn coalesces_to_latest_target_after_current_sync_completes() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");
    let second = PathBuf::from("cache/second.png");

    assert_eq!(
        state.request(Some(first.as_path())).as_deref(),
        Some(first.as_path())
    );
    assert_eq!(state.request(Some(second.as_path())), None);

    let restarted = state.finish_success(first.clone());

    assert_eq!(restarted.as_deref(), Some(second.as_path()));
    assert_eq!(state.active_png_path_for_test(), Some(second.as_path()));
    assert_eq!(state.synced_png_path_for_test(), Some(first.as_path()));
}

#[test]
fn clearing_requested_target_drops_synced_state_after_active_sync_finishes() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    assert_eq!(
        state.request(Some(first.as_path())).as_deref(),
        Some(first.as_path())
    );
    assert_eq!(state.request(None), None);
    assert_eq!(state.finish_success(first), None);
    assert_eq!(state.active_png_path_for_test(), None);
    assert_eq!(state.synced_png_path_for_test(), None);
    assert!(!state.is_busy());
}

#[test]
fn already_synced_target_is_not_restarted() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    assert_eq!(
        state.request(Some(first.as_path())).as_deref(),
        Some(first.as_path())
    );
    assert_eq!(state.finish_success(first.clone()), None);

    let restarted = state.request(Some(first.as_path()));

    assert_eq!(restarted, None);
    assert_eq!(state.active_png_path_for_test(), None);
    assert_eq!(state.synced_png_path_for_test(), Some(first.as_path()));
}
