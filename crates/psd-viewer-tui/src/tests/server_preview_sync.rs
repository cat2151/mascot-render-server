use std::path::PathBuf;

use mascot_render_protocol::PreviewTargetRequest;

use crate::server_preview_sync::ServerPreviewSyncState;

#[test]
fn starts_server_sync_immediately_for_first_target() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    let started = state.request(Some(preview_target("cache/first.png", Some(0.145))));

    assert_eq!(
        started.as_ref().map(|target| target.png_path.as_path()),
        Some(first.as_path())
    );
    assert_eq!(state.active_png_path_for_test(), Some(first.as_path()));
    assert!(state.is_busy());
}

#[test]
fn coalesces_to_latest_target_after_current_sync_completes() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");
    let second = PathBuf::from("cache/second.png");

    assert_eq!(
        state
            .request(Some(preview_target("cache/first.png", Some(0.145))))
            .as_ref()
            .map(|target| target.png_path.as_path()),
        Some(first.as_path())
    );
    assert_eq!(
        state.request(Some(preview_target("cache/second.png", Some(0.145)))),
        None
    );

    let restarted = state.finish_success(preview_target("cache/first.png", Some(0.145)));

    assert_eq!(
        restarted.as_ref().map(|target| target.png_path.as_path()),
        Some(second.as_path())
    );
    assert_eq!(state.active_png_path_for_test(), Some(second.as_path()));
    assert_eq!(state.synced_png_path_for_test(), Some(first.as_path()));
}

#[test]
fn clearing_requested_target_drops_synced_state_after_active_sync_finishes() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    assert_eq!(
        state
            .request(Some(preview_target("cache/first.png", Some(0.145))))
            .as_ref()
            .map(|target| target.png_path.as_path()),
        Some(first.as_path())
    );
    assert_eq!(state.request(None), None);
    assert_eq!(
        state.finish_success(preview_target("cache/first.png", Some(0.145))),
        None
    );
    assert_eq!(state.active_png_path_for_test(), None);
    assert_eq!(state.synced_png_path_for_test(), None);
    assert!(!state.is_busy());
}

#[test]
fn already_synced_target_is_not_restarted() {
    let mut state = ServerPreviewSyncState::default();
    let first = PathBuf::from("cache/first.png");

    assert_eq!(
        state
            .request(Some(preview_target("cache/first.png", Some(0.145))))
            .as_ref()
            .map(|target| target.png_path.as_path()),
        Some(first.as_path())
    );
    assert_eq!(
        state.finish_success(preview_target("cache/first.png", Some(0.145))),
        None
    );

    let restarted = state.request(Some(preview_target("cache/first.png", Some(0.145))));

    assert_eq!(restarted, None);
    assert_eq!(state.active_png_path_for_test(), None);
    assert_eq!(state.synced_png_path_for_test(), Some(first.as_path()));
}

#[test]
fn scale_change_restarts_even_when_png_path_is_same() {
    let mut state = ServerPreviewSyncState::default();

    assert!(state
        .request(Some(preview_target("cache/first.png", Some(0.145))))
        .is_some());
    assert_eq!(
        state.finish_success(preview_target("cache/first.png", Some(0.145))),
        None
    );

    let restarted = state.request(Some(preview_target("cache/first.png", Some(0.239))));

    assert_eq!(
        restarted,
        Some(preview_target("cache/first.png", Some(0.239)))
    );
}

fn preview_target(png_path: &str, scale: Option<f32>) -> PreviewTargetRequest {
    PreviewTargetRequest {
        png_path: PathBuf::from(png_path),
        scale,
        zip_path: PathBuf::from("assets/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/body.psd"),
        display_diff_path: None,
    }
}
