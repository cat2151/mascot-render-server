use std::path::PathBuf;

use crate::{
    mascot_render_server_address, preview_mouth_flap_timeline_request, ChangeSkinRequest,
    MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep, MASCOT_RENDER_SERVER_PORT,
    PREVIEW_MOUTH_FLAP_DURATION_MS, PREVIEW_MOUTH_FLAP_FPS,
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
fn preview_mouth_flap_request_matches_psd_viewer_timing() {
    assert_eq!(
        preview_mouth_flap_timeline_request(),
        MotionTimelineRequest {
            steps: vec![MotionTimelineStep {
                kind: MotionTimelineKind::MouthFlap,
                duration_ms: PREVIEW_MOUTH_FLAP_DURATION_MS,
                fps: PREVIEW_MOUTH_FLAP_FPS,
            }],
        }
    );
    assert_eq!(PREVIEW_MOUTH_FLAP_DURATION_MS, 5_000);
    assert_eq!(PREVIEW_MOUTH_FLAP_FPS, 4);
}
