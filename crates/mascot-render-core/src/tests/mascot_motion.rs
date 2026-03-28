use std::time::{Duration, Instant};

use eframe::egui::{Pos2, Rect, Vec2};

use crate::{
    BounceAnimationConfig, HeadHitbox, IdleSinkAnimationConfig, MotionState, MotionTransform,
    SquashBounceAnimationConfig,
};

#[test]
fn head_hitbox_uses_normalized_coordinates() {
    let hitbox = HeadHitbox {
        x: 0.2,
        y: 0.1,
        width: 0.5,
        height: 0.4,
    };
    let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(200.0, 400.0));

    assert!(hitbox.contains(rect, Pos2::new(70.0, 100.0)));
    assert!(!hitbox.contains(rect, Pos2::new(25.0, 30.0)));
}

#[test]
fn motion_state_alternates_between_animation_kinds() {
    let mut motion = MotionState::new();
    let now = Instant::now();

    assert_eq!(motion.next_animation_name(), "bounce");
    motion.trigger(now);
    assert_eq!(motion.next_animation_name(), "squash_bounce");
    motion.trigger(now + Duration::from_millis(10));
    assert_eq!(motion.next_animation_name(), "bounce");
}

#[test]
fn bounce_transform_moves_image_upward() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    motion.trigger(now);

    let transform = motion.sample(
        now + Duration::from_millis(180),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert!(transform.offset_y < 0.0);
    assert_eq!(transform.scale_x, 1.0);
    assert_eq!(transform.scale_y, 1.0);
}

#[test]
fn squash_bounce_changes_scale() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    motion.trigger(now);
    let _ = motion.sample(
        now + Duration::from_millis(50),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );
    motion.trigger(now + Duration::from_millis(60));

    let transform = motion.sample(
        now + Duration::from_millis(180),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert!(transform.scale_x > 1.0);
    assert!(transform.scale_y < 1.0);
}

#[test]
fn shake_motion_holds_each_random_offset_for_50ms() {
    let mut motion = MotionState::new_with_seed(7);
    let now = Instant::now();
    motion.trigger_shake(now, 20.0, Duration::from_secs(5), 20);

    let first = motion.sample(
        now + Duration::from_millis(25),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );
    let same_frame = motion.sample(
        now + Duration::from_millis(49),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );
    let next_frame = motion.sample(
        now + Duration::from_millis(50),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert_eq!(first, same_frame);
    assert_ne!(next_frame, MotionTransform::identity());
    assert_ne!(first, next_frame);
    assert!(first.offset_x.abs() <= 20.0);
    assert!(first.offset_y.abs() <= 20.0);
    assert_eq!(first.scale_x, 1.0);
    assert_eq!(first.scale_y, 1.0);
}

#[test]
fn shake_motion_finishes_after_requested_duration() {
    let mut motion = MotionState::new_with_seed(11);
    let now = Instant::now();
    motion.trigger_shake(now, 20.0, Duration::from_secs(5), 20);

    let running = motion.sample(
        now + Duration::from_millis(4_999),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );
    let finished = motion.sample(
        now + Duration::from_millis(5_000),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert_ne!(running, MotionTransform::identity());
    assert_eq!(finished, MotionTransform::identity());
    assert!(!motion.is_active());
}

#[test]
fn mouth_flap_motion_starts_open_and_switches_each_frame() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    motion.trigger_mouth_flap(now, Duration::from_secs(5), 4);

    assert_eq!(motion.mouth_flap_is_open(now), Some(true));
    assert_eq!(
        motion.mouth_flap_is_open(now + Duration::from_millis(249)),
        Some(true)
    );
    assert_eq!(
        motion.mouth_flap_is_open(now + Duration::from_millis(250)),
        Some(false)
    );
    assert_eq!(
        motion.mouth_flap_is_open(now + Duration::from_millis(500)),
        Some(true)
    );
}

#[test]
fn mouth_flap_motion_finishes_after_requested_duration() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    motion.trigger_mouth_flap(now, Duration::from_secs(5), 20);

    assert_eq!(
        motion.mouth_flap_is_open(now + Duration::from_millis(4_999)),
        Some(false)
    );
    assert_eq!(
        motion.mouth_flap_is_open(now + Duration::from_millis(5_000)),
        None
    );
    assert!(!motion.is_active());
}

#[test]
fn always_bouncing_idle_uses_always_idle_sink_duration() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    let bounce = BounceAnimationConfig::default();
    let squash_bounce = SquashBounceAnimationConfig {
        duration_ms: 40,
        ..SquashBounceAnimationConfig::default()
    };
    let always_idle_sink = IdleSinkAnimationConfig {
        duration_ms: 100,
        ..IdleSinkAnimationConfig::default_for_always_bouncing()
    };

    motion.set_always_idle_sink_enabled(true, now);

    let running = motion.sample(
        now + Duration::from_millis(60),
        bounce,
        squash_bounce,
        always_idle_sink,
    );
    let restarted = motion.sample(
        now + Duration::from_millis(100),
        bounce,
        squash_bounce,
        always_idle_sink,
    );
    let second_cycle = motion.sample(
        now + Duration::from_millis(140),
        bounce,
        squash_bounce,
        always_idle_sink,
    );

    assert_ne!(running, MotionTransform::identity());
    assert_eq!(restarted, MotionTransform::identity());
    assert_ne!(second_cycle, MotionTransform::identity());
    assert!(motion.is_active());
}

#[test]
fn always_bouncing_triggered_animation_ignores_always_idle_sink() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    let bounce = BounceAnimationConfig::default();
    let squash_bounce = SquashBounceAnimationConfig {
        duration_ms: 100,
        ..SquashBounceAnimationConfig::default()
    };
    let always_idle_sink = IdleSinkAnimationConfig {
        duration_ms: 500,
        ..IdleSinkAnimationConfig::default_for_always_bouncing()
    };

    motion.set_always_idle_sink_enabled(true, now);
    motion.trigger(now + Duration::from_millis(1));
    motion.trigger(now + Duration::from_millis(2));

    let transform = motion.sample(
        now + Duration::from_millis(150),
        bounce,
        squash_bounce,
        always_idle_sink,
    );

    assert_eq!(transform, MotionTransform::identity());
}

#[test]
fn disabling_always_bouncing_stops_idle_animation() {
    let mut motion = MotionState::new();
    let now = Instant::now();

    motion.set_always_idle_sink_enabled(true, now);
    assert!(motion.is_active());

    motion.set_always_idle_sink_enabled(false, now + Duration::from_millis(10));

    let transform = motion.sample(
        now + Duration::from_millis(20),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert_eq!(transform, MotionTransform::identity());
    assert!(!motion.is_active());
}

#[test]
fn disabling_always_bouncing_keeps_triggered_squash_bounce_running() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    let bounce = BounceAnimationConfig::default();
    let squash_bounce = SquashBounceAnimationConfig {
        duration_ms: 100,
        ..SquashBounceAnimationConfig::default()
    };

    motion.set_always_idle_sink_enabled(true, now);
    motion.trigger(now + Duration::from_millis(1));
    motion.trigger(now + Duration::from_millis(2));
    motion.set_always_idle_sink_enabled(false, now + Duration::from_millis(10));

    let transform = motion.sample(
        now + Duration::from_millis(20),
        bounce,
        squash_bounce,
        IdleSinkAnimationConfig::default_for_always_bouncing(),
    );

    assert!(transform.scale_x > 1.0);
    assert!(motion.is_active());
}

#[test]
fn idle_sink_starts_at_rest_and_then_scales_for_sink_and_lift() {
    let mut motion = MotionState::new();
    let now = Instant::now();
    let idle_sink = IdleSinkAnimationConfig {
        duration_ms: 400,
        ..IdleSinkAnimationConfig::default_for_always_bouncing()
    };

    motion.set_always_idle_sink_enabled(true, now);

    let resting = motion.sample(
        now,
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        idle_sink,
    );
    let sinking = motion.sample(
        now + Duration::from_millis(100),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        idle_sink,
    );
    let lifting = motion.sample(
        now + Duration::from_millis(300),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
        idle_sink,
    );

    assert_eq!(resting, MotionTransform::identity());
    assert_eq!(sinking.offset_y, 0.0);
    assert!(sinking.scale_x > 1.0);
    assert!(sinking.scale_y < 1.0);
    assert_eq!(lifting.offset_y, 0.0);
    assert!(lifting.scale_x < 1.0);
    assert!(lifting.scale_y > 1.0);
}
