use std::path::PathBuf;

use crate::{
    mascot_render_server_address, ChangeSkinRequest, MotionTimelineKind, MotionTimelineRequest,
    MotionTimelineStep, MASCOT_RENDER_SERVER_PORT,
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
fn default_server_address_uses_expected_port() {
    assert_eq!(
        mascot_render_server_address().port(),
        MASCOT_RENDER_SERVER_PORT
    );
}

#[test]
fn motion_timeline_request_round_trips_as_json() {
    let request = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::MouthFlap,
            duration_ms: 5_000,
            fps: 20,
        }],
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: MotionTimelineRequest =
        serde_json::from_str(&json).expect("request should deserialize");

    assert_eq!(decoded, request);
}
