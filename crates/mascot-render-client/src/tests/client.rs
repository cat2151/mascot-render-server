use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

use crate::{
    mascot_render_server_address, preview_mouth_flap_timeline_request, MASCOT_RENDER_SERVER_PORT,
    PREVIEW_MOUTH_FLAP_DURATION_MS, PREVIEW_MOUTH_FLAP_FPS,
};

#[test]
fn default_server_address_uses_expected_port() {
    assert_eq!(
        mascot_render_server_address().port(),
        MASCOT_RENDER_SERVER_PORT
    );
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
