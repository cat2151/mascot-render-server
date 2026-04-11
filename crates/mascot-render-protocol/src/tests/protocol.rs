use std::path::PathBuf;

use crate::{
    validate_motion_timeline_request, ChangeSkinRequest, MotionTimelineKind, MotionTimelineRequest,
    MotionTimelineStep, ServerCommandKind, ServerCommandStage, ServerCommandStatus,
    ServerLifecyclePhase, ServerStatusSnapshot, ServerStatusStore,
};

#[test]
fn change_skin_request_round_trips_as_json() {
    let request = ChangeSkinRequest {
        png_path: PathBuf::from("cache/demo/variation.png"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: ChangeSkinRequest =
        serde_json::from_str(&json).expect("request should deserialize");

    assert_eq!(decoded, request);
}

#[test]
fn motion_timeline_request_round_trips_as_json() {
    for kind in [MotionTimelineKind::Shake, MotionTimelineKind::MouthFlap] {
        let request = MotionTimelineRequest {
            steps: vec![MotionTimelineStep {
                kind,
                duration_ms: 5_000,
                fps: 20,
            }],
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: MotionTimelineRequest =
            serde_json::from_str(&json).expect("request should deserialize");

        assert_eq!(decoded, request);
    }
}

#[test]
fn validate_motion_timeline_request_rejects_empty_timeline() {
    let request = MotionTimelineRequest { steps: vec![] };

    let error = validate_motion_timeline_request(&request)
        .expect_err("empty motion timeline should be rejected");

    assert!(
        error.to_string().contains("exactly one step"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn validate_motion_timeline_request_rejects_zero_duration() {
    let request = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: 0,
            fps: 20,
        }],
    };

    let error = validate_motion_timeline_request(&request)
        .expect_err("zero-duration motion timeline should be rejected");

    assert!(
        error
            .to_string()
            .contains("duration must be greater than zero"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn validate_motion_timeline_request_accepts_single_step() {
    let request = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::MouthFlap,
            duration_ms: 5_000,
            fps: 20,
        }],
    };

    validate_motion_timeline_request(&request)
        .expect("single-step motion timeline should be accepted");
}

#[test]
fn server_status_snapshot_round_trips_as_json() {
    let mut snapshot = ServerStatusSnapshot::starting(
        PathBuf::from("config/mascot-render-server.toml"),
        PathBuf::from("config/mascot-render-server.runtime.json"),
        PathBuf::from("cache/demo/open.png"),
    );
    snapshot.lifecycle = ServerLifecyclePhase::Running;
    snapshot.current_command = Some(ServerCommandStatus::queued(
        ServerCommandKind::ChangeSkin,
        "to=cache/demo/open.png",
    ));

    let json = serde_json::to_string(&snapshot).expect("snapshot should serialize");
    let decoded: ServerStatusSnapshot =
        serde_json::from_str(&json).expect("snapshot should deserialize");

    assert_eq!(decoded, snapshot);
}

#[test]
fn server_command_status_updates_stage_without_losing_request_time() {
    let queued = ServerCommandStatus::queued(ServerCommandKind::Timeline, "shake");
    let applied = queued.with_stage(
        ServerCommandStage::Applied,
        queued.updated_at_unix_ms + 1,
        None,
    );

    assert_eq!(applied.kind, queued.kind);
    assert_eq!(applied.summary, queued.summary);
    assert_eq!(applied.requested_at_unix_ms, queued.requested_at_unix_ms);
    assert_eq!(applied.stage, ServerCommandStage::Applied);
}

#[test]
fn server_status_store_returns_updated_snapshot() {
    let store = ServerStatusStore::new(ServerStatusSnapshot::starting(
        PathBuf::from("config.toml"),
        PathBuf::from("runtime.json"),
        PathBuf::from("skin.png"),
    ));

    store
        .update(|snapshot| snapshot.lifecycle = ServerLifecyclePhase::Running)
        .expect("store update should succeed");

    assert_eq!(
        store
            .snapshot()
            .expect("snapshot should be readable")
            .lifecycle,
        ServerLifecyclePhase::Running
    );
}
