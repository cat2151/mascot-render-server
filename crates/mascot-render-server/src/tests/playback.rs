use crate::mascot_app::playback::transform;
use mascot_render_core::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionState, MotionTransform,
    SquashBounceAnimationConfig,
};
use mascot_render_protocol::{PlaybackRequest, PlaybackSnapshot};
use std::time::{Duration, Instant};

fn snapshot(now: Instant, rate: f64) -> PlaybackSnapshot {
    PlaybackSnapshot {
        request: PlaybackRequest {
            sent_unix_ms: 0,
            playing: true,
            beat: Some(0.0),
            beats_per_second: rate,
            singing: true,
        },
        received: now,
        expires: now + Duration::from_secs(4),
        mouth_started: now,
    }
}

fn click_motion(elapsed: Duration, config: SquashBounceAnimationConfig) -> MotionTransform {
    let start = Instant::now();
    let mut motion = MotionState::new();
    motion.trigger(start); // bounce
    motion.trigger(start); // the alternating squash reaction
    motion.sample(
        start + elapsed,
        BounceAnimationConfig::default(),
        config,
        IdleSinkAnimationConfig::default(),
    )
}

fn assert_transform(actual: MotionTransform, expected: MotionTransform) {
    assert!((actual.scale_x - expected.scale_x).abs() < 0.00001);
    assert!((actual.scale_y - expected.scale_y).abs() < 0.00001);
    assert!((actual.offset_x - expected.offset_x).abs() < 0.00001);
    assert!((actual.offset_y - expected.offset_y).abs() < 0.00001);
}

#[test]
fn original_click_motion_is_cut_off_at_500ms_and_restarted_without_blending() {
    let now = Instant::now();
    let state = snapshot(now, 2.0);
    // Keep a motion longer than a beat to exercise the intentional cutoff.
    let config = SquashBounceAnimationConfig {
        stretch_amount: 0.05,
        ..SquashBounceAnimationConfig::default()
    };
    for ms in [0, 50, 125, 250, 499, 500, 501, 625, 999, 1000] {
        let at = now + Duration::from_millis(ms);
        let expected = click_motion(Duration::from_millis(ms % 500), config);
        assert_transform(transform(Some(&state), at, config).unwrap(), expected);
    }
    let before = transform(Some(&state), now + Duration::from_millis(499), config).unwrap();
    let restart = transform(Some(&state), now + Duration::from_millis(500), config).unwrap();
    assert!((before.scale_x - restart.scale_x).abs() > 0.1);
    assert_eq!(restart.offset_y, 0.0);
    assert!((restart.scale_x - 1.16).abs() < 0.00001);
    assert!((restart.scale_y - 0.95).abs() < 0.00001);
    assert!(
        (click_motion(Duration::ZERO, SquashBounceAnimationConfig::default()).scale_y - 0.90).abs()
            < 0.00001
    );
}

#[test]
fn playback_uses_custom_settings_and_preserves_wall_time_at_other_tempos_and_seek() {
    let now = Instant::now();
    let config = SquashBounceAnimationConfig {
        amplitude_px: 47.0,
        cycles: 2.3,
        damping: 1.7,
        squash_amount: 0.23,
        stretch_amount: 0.12,
        ..SquashBounceAnimationConfig::default()
    };
    for rate in [1.0, 2.0, 4.0] {
        let state = snapshot(now, rate);
        assert_transform(
            transform(Some(&state), now + Duration::from_millis(125), config).unwrap(),
            click_motion(Duration::from_millis(125), config),
        );
    }
    let mut state = snapshot(now, 2.0);
    state.request.beat = Some(8.25);
    assert_transform(
        transform(Some(&state), now, config).unwrap(),
        click_motion(Duration::from_millis(125), config),
    );
    state.request.beat = Some(0.25); // A-B repeat rebases the position.
    assert_transform(
        transform(Some(&state), now, config).unwrap(),
        click_motion(Duration::from_millis(125), config),
    );
}

#[test]
fn slow_beats_rest_after_original_duration_and_inactive_states_release_override() {
    let now = Instant::now();
    let mut state = snapshot(now, 1.0);
    let config = SquashBounceAnimationConfig::default_for_musical();
    for ms in [500, 760, 900, 999] {
        assert_transform(
            transform(Some(&state), now + Duration::from_millis(ms), config).unwrap(),
            MotionTransform::identity(),
        );
    }
    assert!(transform(Some(&state), state.expires, config).is_none());
    state.request.playing = false;
    assert!(transform(Some(&state), now, config).is_none());
    state.request.playing = true;
    state.request.beat = None;
    assert!(transform(Some(&state), now, config).is_none());
    assert!(transform(None, now, config).is_none());
}
