use std::path::PathBuf;

use mascot_render_protocol::{ServerStatusSnapshot, ServerStatusStore};

use crate::mascot_app::ServerWorkGuard;

#[test]
fn server_work_guard_records_and_clears_current_work() {
    let store = ServerStatusStore::new(sample_snapshot());

    {
        let _work = ServerWorkGuard::start_for_test(
            store.clone(),
            "reload_config_if_needed",
            "load_mascot_config",
            "config_path=config.toml",
        );

        let snapshot = store.snapshot().expect("snapshot should be readable");
        let current_work = snapshot
            .current_work
            .expect("current work should be recorded");
        assert_eq!(current_work.kind, "reload_config_if_needed");
        assert_eq!(current_work.stage, "load_mascot_config");
        assert_eq!(current_work.summary, "config_path=config.toml");
    }

    assert!(
        store
            .snapshot()
            .expect("snapshot should be readable")
            .current_work
            .is_none(),
        "current work should clear when guard is dropped"
    );
}

#[test]
fn server_work_guard_restores_nested_previous_work() {
    let store = ServerStatusStore::new(sample_snapshot());

    let mut outer = ServerWorkGuard::start_for_test(
        store.clone(),
        "reload_config_if_needed",
        "load_active_skin",
        "png_path=open.png",
    );
    {
        let _inner = ServerWorkGuard::start_for_test(
            store.clone(),
            "load_skin",
            "cache_miss_decode_texture",
            "png_path=open.png",
        );
        assert_eq!(
            store
                .snapshot()
                .expect("snapshot should be readable")
                .current_work
                .expect("inner current work should be visible")
                .kind,
            "load_skin"
        );
    }

    outer.update_stage("refresh_window_layout", "png_changed=true");
    let snapshot = store.snapshot().expect("snapshot should be readable");
    let current_work = snapshot
        .current_work
        .expect("outer current work should be restored");
    assert_eq!(current_work.kind, "reload_config_if_needed");
    assert_eq!(current_work.stage, "refresh_window_layout");
}

fn sample_snapshot() -> ServerStatusSnapshot {
    ServerStatusSnapshot::starting(
        PathBuf::from("config.toml"),
        PathBuf::from("runtime.json"),
        PathBuf::from("skin.png"),
        PathBuf::from("demo.zip"),
        PathBuf::from("demo.psd"),
    )
}
