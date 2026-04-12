use std::path::PathBuf;

use mascot_render_control::MascotControlCommand;
use mascot_render_protocol::{
    MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep, ServerStatusSnapshot,
    ServerStatusStore,
};

use crate::mascot_app::{PendingPerformanceTrace, ServerWorkGuard};

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

#[test]
fn pending_performance_trace_formats_completed_change_character_latency() {
    let command = MascotControlCommand::change_character("demo".to_string());
    let mut trace = PendingPerformanceTrace::from_command_for_test(
        &command,
        PathBuf::from("cache/old/open.png"),
    )
    .expect("change-character should be measured");
    trace.record_stage_for_test("resolve_character_skin", 11);
    trace.record_stage_for_test("refresh_mouth_flap_skins", 29);
    trace.mark_applied_for_test(command.status().requested_at_unix_ms + 37);

    let message = trace.completed_message_for_test(
        command.status().requested_at_unix_ms + 42,
        PathBuf::from("cache/new/open.png").as_path(),
        PathBuf::from("cache/new/open.png").as_path(),
        PathBuf::from("cache/old/open.png").as_path(),
    );

    assert!(message.contains("action=change_character"));
    assert!(message.contains("result=completed"));
    assert!(message.contains("elapsed_ms=42"));
    assert!(message.contains("queue_ms=7"));
    assert!(message.contains("apply_ms=30"));
    assert!(message.contains("settle_ms=5"));
    assert!(message.contains("stage_ms=resolve_character_skin:11ms,refresh_mouth_flap_skins:29ms"));
    assert!(message.contains("status_settled=true"));
    assert!(message.contains("texture_changed=true"));
}

#[test]
fn pending_performance_trace_ignores_non_texture_timeline() {
    let command = MascotControlCommand::play_timeline(MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: 900,
            fps: 20,
        }],
    });

    assert!(
        PendingPerformanceTrace::from_command_for_test(&command, PathBuf::from("cache/open.png"))
            .is_none(),
        "shake does not change texture and should not produce texture performance logs"
    );
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
