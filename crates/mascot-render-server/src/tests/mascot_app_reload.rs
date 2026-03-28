use std::time::{Duration, Instant, SystemTime};

use crate::mascot_app::should_reload_config_for_test;

#[test]
fn reload_check_reacts_to_psd_viewer_tui_activity_changes_immediately() {
    let same = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    let changed = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(11));
    let now = Instant::now();

    let should_reload = should_reload_config_for_test(
        [same, same, same, same, same],
        [same, same, same, changed, same],
        now,
        now,
    );

    assert!(should_reload);
}

#[test]
fn reload_check_polls_even_when_files_are_unchanged() {
    let same = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    let now = Instant::now();

    let should_reload = should_reload_config_for_test(
        [same, same, same, same, same],
        [same, same, same, same, same],
        now - Duration::from_secs(1),
        now,
    );

    assert!(should_reload);
}
