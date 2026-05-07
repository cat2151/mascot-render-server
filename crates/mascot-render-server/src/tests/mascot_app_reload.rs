use std::time::{Duration, Instant, SystemTime};

use mascot_render_protocol::PlacementMode;

use crate::mascot_app::{
    pending_scale_persist_remaining_at_for_test, should_refresh_auxiliary_skins_now_for_test,
    should_reload_config_for_test, should_restore_window_history_for_reload_for_test,
};

#[test]
fn reload_check_reacts_to_psd_viewer_tui_activity_changes_immediately() {
    let same = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    let changed = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(11));

    let should_reload = should_reload_config_for_test(
        [same, same, same, same, same],
        [same, same, same, changed, same],
    );

    assert!(should_reload);
}

#[test]
fn reload_check_ignores_unchanged_files() {
    let same = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));

    let should_reload = should_reload_config_for_test(
        [same, same, same, same, same],
        [same, same, same, same, same],
    );

    assert!(!should_reload);
}

#[test]
fn auxiliary_skin_refresh_waits_until_after_config_reload_frame() {
    assert!(!should_refresh_auxiliary_skins_now_for_test(true, true));
    assert!(should_refresh_auxiliary_skins_now_for_test(false, true));
    assert!(!should_refresh_auxiliary_skins_now_for_test(false, false));
}

#[test]
fn shared_visual_size_reload_does_not_restore_per_psd_window_history() {
    assert!(!should_restore_window_history_for_reload_for_test(
        PlacementMode::SharedVisualSize,
        true,
        false,
    ));
    assert!(!should_restore_window_history_for_reload_for_test(
        PlacementMode::SharedVisualSize,
        false,
        true,
    ));
    assert!(should_restore_window_history_for_reload_for_test(
        PlacementMode::PerPsd,
        true,
        false,
    ));
    assert!(!should_restore_window_history_for_reload_for_test(
        PlacementMode::PerPsd,
        false,
        false,
    ));
}

#[test]
fn latest_scale_input_resets_persist_debounce_before_due_write_runs() {
    let now = Instant::now();
    let expired_change_at = now - Duration::from_millis(250);

    assert_eq!(
        pending_scale_persist_remaining_at_for_test(Some(0.37), Some(expired_change_at), now),
        Some(Duration::ZERO)
    );
    assert_eq!(
        pending_scale_persist_remaining_at_for_test(Some(0.27), Some(now), now),
        Some(Duration::from_millis(250))
    );
}
