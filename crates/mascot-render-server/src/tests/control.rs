use mascot_render_core::MascotEnsembleMode;
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

use crate::mascot_app::{
    should_consume_targeted_bounce_timeline_for_test,
    should_consume_targeted_mouth_flap_timeline_for_test,
};

#[test]
fn targeted_mouth_flap_falls_through_in_single_character_mode() {
    assert!(!should_consume_targeted_mouth_flap_timeline_for_test(
        &mouth_flap_request(),
        MascotEnsembleMode::SingleCharacter,
    ));
}

#[test]
fn targeted_mouth_flap_falls_through_in_favorite_mode() {
    assert!(!should_consume_targeted_mouth_flap_timeline_for_test(
        &mouth_flap_request(),
        MascotEnsembleMode::Favorite,
    ));
}

#[test]
fn targeted_mouth_flap_is_consumed_in_vpt_mode() {
    assert!(should_consume_targeted_mouth_flap_timeline_for_test(
        &mouth_flap_request(),
        MascotEnsembleMode::Vpt,
    ));
}

#[test]
fn targeted_shake_falls_through_even_in_vpt_mode() {
    assert!(!should_consume_targeted_mouth_flap_timeline_for_test(
        &MotionTimelineRequest {
            steps: vec![MotionTimelineStep {
                kind: MotionTimelineKind::Shake,
                duration_ms: 250,
                fps: 20,
            }],
            target_character_name: Some("ずんだもん".to_string()),
        },
        MascotEnsembleMode::Vpt,
    ));
}

#[test]
fn targeted_bounce_is_consumed_in_vpt_mode() {
    assert!(should_consume_targeted_bounce_timeline_for_test(
        &bounce_request(),
        MascotEnsembleMode::Vpt,
    ));
}

#[test]
fn targeted_bounce_falls_through_in_single_character_mode() {
    assert!(!should_consume_targeted_bounce_timeline_for_test(
        &bounce_request(),
        MascotEnsembleMode::SingleCharacter,
    ));
}

#[test]
fn targeted_bounce_without_target_falls_through_even_in_vpt_mode() {
    let mut request = bounce_request();
    request.target_character_name = None;

    assert!(!should_consume_targeted_bounce_timeline_for_test(
        &request,
        MascotEnsembleMode::Vpt,
    ));
}

#[test]
fn mouth_flap_without_target_falls_through_even_in_vpt_mode() {
    let mut request = mouth_flap_request();
    request.target_character_name = None;

    assert!(!should_consume_targeted_mouth_flap_timeline_for_test(
        &request,
        MascotEnsembleMode::Vpt,
    ));
}

fn mouth_flap_request() -> MotionTimelineRequest {
    MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::MouthFlap,
            duration_ms: 250,
            fps: 20,
        }],
        target_character_name: Some("ずんだもん".to_string()),
    }
}

fn bounce_request() -> MotionTimelineRequest {
    MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Bounce,
            duration_ms: 900,
            fps: 60,
        }],
        target_character_name: Some("ずんだもん".to_string()),
    }
}
