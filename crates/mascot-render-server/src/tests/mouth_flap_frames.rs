use crate::mouth_flap_frames::{select_frame, MouthFlapFrames};

fn frames(blink: bool) -> MouthFlapFrames<&'static str> {
    MouthFlapFrames {
        open: "mouth-open",
        closed: "mouth-closed",
        blink_open: blink.then_some("blink-mouth-open"),
        blink_closed: blink.then_some("blink-mouth-closed"),
    }
}

fn select(
    mouth_flap: Option<&MouthFlapFrames<&'static str>>,
    mouth_open: Option<bool>,
    blink_closed: bool,
) -> &'static str {
    select_frame(
        &"open",
        Some(&"blink"),
        mouth_flap,
        mouth_open,
        blink_closed,
    )
}

#[test]
fn blink_keeps_the_current_mouth_frame_during_mouth_flap() {
    let frames = frames(true);
    assert_eq!(select(Some(&frames), Some(true), true), "blink-mouth-open");
    assert_eq!(
        select(Some(&frames), Some(false), true),
        "blink-mouth-closed"
    );
    assert_eq!(select(Some(&frames), Some(true), false), "mouth-open");
    assert_eq!(select(Some(&frames), Some(false), false), "mouth-closed");
}

#[test]
fn without_mouth_flap_blink_uses_the_closed_eye_skin() {
    let frames = frames(true);
    assert_eq!(select(Some(&frames), None, true), "blink");
    assert_eq!(select(Some(&frames), None, false), "open");
    assert_eq!(select(None, Some(true), true), "blink");
    assert_eq!(select(None, None, false), "open");
}

#[test]
fn missing_closed_eye_mouth_frames_keep_the_mouth_moving() {
    let frames = frames(false);
    assert_eq!(select(Some(&frames), Some(true), true), "mouth-open");
    assert_eq!(select(Some(&frames), Some(false), true), "mouth-closed");
}
