use mascot_render_protocol::MotionTimelineKind;

use crate::actions::{shake_timeline_request, TestPostAction};

#[test]
fn test_post_action_labels_match_key_descriptions() {
    assert_eq!(TestPostAction::Show.label(), "show");
    assert_eq!(TestPostAction::Hide.label(), "hide");
    assert_eq!(
        TestPostAction::change_skin_label(),
        "change-skin current_png_path"
    );
    assert_eq!(TestPostAction::ShakeTimeline.label(), "timeline shake");
    assert_eq!(
        TestPostAction::MouthFlapTimeline.label(),
        "timeline mouth-flap"
    );
}

#[test]
fn shake_timeline_request_uses_single_test_step() {
    let request = shake_timeline_request();

    assert_eq!(request.steps.len(), 1);
    assert_eq!(request.steps[0].kind, MotionTimelineKind::Shake);
    assert_eq!(request.steps[0].duration_ms, 900);
    assert_eq!(request.steps[0].fps, 20);
}
