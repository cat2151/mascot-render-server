use std::fs;
use std::time::Instant;

use mascot_render_core::workspace_cache_root;

use crate::activity_heartbeat::ActivityHeartbeat;

#[test]
fn activity_heartbeat_creates_and_removes_activity_file() {
    let path = workspace_cache_root().join("test-psd-viewer-tui-activity/heartbeat");
    let _ = fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }

    {
        let _heartbeat = ActivityHeartbeat::start_with_path_for_test(path.clone(), Instant::now())
            .expect("should create psd-viewer-tui activity heartbeat");
        assert!(
            path.exists(),
            "activity heartbeat should exist while psd-viewer-tui is running"
        );
    }

    assert!(
        !path.exists(),
        "activity heartbeat should be removed when psd-viewer-tui stops"
    );
}
