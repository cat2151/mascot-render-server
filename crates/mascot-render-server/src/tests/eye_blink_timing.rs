use std::time::{Duration, Instant};

use mascot_render_core::SquashBounceAnimationConfig;

use crate::eye_blink_timing::{always_squash_bounce_for_blink_median, EyeBlinkIntervalGenerator};

#[test]
fn eye_blink_interval_generator_clamps_to_requested_bounds() {
    let now = Instant::now();
    let mut generator = EyeBlinkIntervalGenerator::new_for_test(now, 7);

    for step in 0..64 {
        let interval_ms = generator.next_interval_ms(now + Duration::from_secs(step));
        assert!(
            (1000..=8000).contains(&interval_ms),
            "interval out of range: {interval_ms}"
        );
    }
}

#[test]
fn eye_blink_interval_generator_keeps_drifted_median_within_twenty_percent() {
    let now = Instant::now();
    let mut generator = EyeBlinkIntervalGenerator::new_for_test(now, 11);

    for step in 1..32 {
        let sample_at = now + Duration::from_millis(step * 700);
        let _ = generator.next_interval_ms(sample_at);
        let median_ms = generator.current_median_ms_for_test();
        assert!(
            (2880.0..=4320.0).contains(&median_ms),
            "median drift out of range: {median_ms}"
        );
    }
}

#[test]
fn always_squash_bounce_duration_tracks_blink_median() {
    let config = SquashBounceAnimationConfig {
        duration_ms: 1000,
        ..SquashBounceAnimationConfig::default_for_always_bouncing()
    };

    let slower = always_squash_bounce_for_blink_median(config, 4320.0);
    let faster = always_squash_bounce_for_blink_median(config, 2880.0);

    assert_eq!(slower.duration_ms, 1200);
    assert_eq!(faster.duration_ms, 800);
    assert_eq!(slower.amplitude_px, config.amplitude_px);
}
