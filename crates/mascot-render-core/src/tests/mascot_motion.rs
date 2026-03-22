use std::time::{Duration, Instant};

use eframe::egui::{Pos2, Rect, Vec2};

use crate::{
    BounceAnimationConfig, HeadHitbox, MotionState, MotionTransform, SquashBounceAnimationConfig,
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
    );
    motion.trigger(now + Duration::from_millis(60));

    let transform = motion.sample(
        now + Duration::from_millis(180),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
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
    );
    let same_frame = motion.sample(
        now + Duration::from_millis(49),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
    );
    let next_frame = motion.sample(
        now + Duration::from_millis(50),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
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
    );
    let finished = motion.sample(
        now + Duration::from_millis(5_000),
        BounceAnimationConfig::default(),
        SquashBounceAnimationConfig::default(),
    );

    assert_ne!(running, MotionTransform::identity());
    assert_eq!(finished, MotionTransform::identity());
    assert!(!motion.is_active());
}
