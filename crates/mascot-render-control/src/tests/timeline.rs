use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

use crate::validate_motion_timeline_request;

#[test]
fn rejects_empty_motion_timeline() {
    let request = MotionTimelineRequest { steps: vec![] };

    let error = validate_motion_timeline_request(&request)
        .expect_err("empty motion timeline should be rejected");

    assert!(
        error.to_string().contains("exactly one step"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn rejects_zero_duration_motion_timeline() {
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
fn accepts_single_step_motion_timeline() {
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
