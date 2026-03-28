use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::app::mouth_flap::MouthFlapAnimation;

#[test]
fn mouth_flap_animation_switches_frames_every_250ms() {
    let now = Instant::now();
    let mut animation = MouthFlapAnimation::new_for_test(
        [PathBuf::from("frame-a.png"), PathBuf::from("frame-b.png")],
        now,
    );

    assert_eq!(animation.current_frame_label_for_test(), "ほあー");
    assert!(!animation.advance_for_test(now + Duration::from_millis(200)));
    assert_eq!(animation.current_frame_label_for_test(), "ほあー");

    assert!(animation.advance_for_test(now + Duration::from_millis(250)));
    assert_eq!(animation.current_frame_label_for_test(), "むふ");

    assert!(animation.advance_for_test(now + Duration::from_millis(500)));
    assert_eq!(animation.current_frame_label_for_test(), "ほあー");
}

#[test]
fn mouth_flap_animation_finishes_after_five_seconds() {
    let now = Instant::now();
    let animation = MouthFlapAnimation::new_for_test(
        [PathBuf::from("frame-a.png"), PathBuf::from("frame-b.png")],
        now,
    );

    assert!(!animation.is_finished_for_test(now + Duration::from_secs(4)));
    assert!(animation.is_finished_for_test(now + Duration::from_secs(5)));
}

#[test]
fn mouth_flap_animation_uses_configured_labels() {
    let now = Instant::now();
    let mut animation = MouthFlapAnimation::new_with_labels_for_test(
        [PathBuf::from("frame-a.png"), PathBuf::from("frame-b.png")],
        ["あ".to_string(), "ん".to_string()],
        now,
    );

    assert_eq!(animation.current_frame_label_for_test(), "あ");
    assert!(animation.advance_for_test(now + Duration::from_millis(250)));
    assert_eq!(animation.current_frame_label_for_test(), "ん");
}
