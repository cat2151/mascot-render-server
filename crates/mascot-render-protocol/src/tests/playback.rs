use crate::playback::*;
use std::time::{Duration, Instant};
fn request(at: u64, singing: bool) -> PlaybackRequest {
    PlaybackRequest {
        sent_unix_ms: at,
        playing: true,
        beat: Some(0.0),
        beats_per_second: 2.0,
        singing,
    }
}
#[test]
fn updates_preserve_mouth_phase_and_expire() {
    let now = Instant::now();
    let mut slot = PlaybackSlot::default();
    slot.set(request(1000, true), now, 1000).unwrap();
    slot.set(request(1300, true), now + Duration::from_millis(300), 1300)
        .unwrap();
    let s = slot.0.as_ref().unwrap();
    assert_eq!(s.mouth_open(now + Duration::from_millis(300)), Some(false));
    assert_eq!(s.beat(now + Duration::from_millis(550)), Some(0.5));
    assert!(!s.is_active(now + Duration::from_millis(1050)));
    slot.set(request(1400, false), now + Duration::from_millis(400), 1400)
        .unwrap();
    assert_eq!(
        slot.0
            .as_ref()
            .unwrap()
            .mouth_open(now + Duration::from_millis(400)),
        None
    );
}
#[test]
fn delayed_update_cannot_resurrect_or_extend_playback() {
    let now = Instant::now();
    let mut slot = PlaybackSlot::default();
    slot.set(request(1000, true), now, 1800).unwrap();
    assert!(slot.0.is_none());
    slot.set(request(2000, true), now, 2250).unwrap();
    let s = slot.0.as_ref().unwrap();
    assert_eq!(s.beat(now), Some(0.5));
    assert!(!s.is_active(now + Duration::from_millis(500)));
    let mut stop = request(2251, false);
    stop.playing = false;
    slot.set(stop, now, 2251).unwrap();
    assert!(!slot.0.as_ref().unwrap().is_active(now));
}
#[test]
fn invalid_timing_is_rejected() {
    let mut r = request(0, true);
    r.beat = Some(f64::NAN);
    assert!(r.validate().is_err());
    r.beat = None;
    r.beats_per_second = 0.0;
    assert!(r.validate().is_err());
}
