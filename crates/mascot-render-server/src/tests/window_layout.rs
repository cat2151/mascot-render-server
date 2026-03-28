use eframe::egui::{Pos2, Rect, Vec2};
use mascot_render_core::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionTransform, SquashBounceAnimationConfig,
};

use crate::{alpha_bounds_from_mask, anchored_inner_origin, AlphaBounds, MascotWindowLayout};

fn zero_bounce() -> BounceAnimationConfig {
    BounceAnimationConfig {
        amplitude_px: 0.0,
        ..Default::default()
    }
}

fn zero_squash() -> SquashBounceAnimationConfig {
    SquashBounceAnimationConfig {
        amplitude_px: 0.0,
        squash_amount: 0.0,
        stretch_amount: 0.0,
        ..Default::default()
    }
}

fn zero_idle_sink() -> IdleSinkAnimationConfig {
    IdleSinkAnimationConfig {
        sink_amount: 0.0,
        lift_amount: 0.0,
        ..Default::default()
    }
}

#[test]
fn alpha_bounds_track_non_transparent_pixels() {
    let alpha_mask = [
        0, 0, 0, 0, //
        0, 9, 7, 0, //
        0, 0, 8, 0, //
    ];

    assert_eq!(
        alpha_bounds_from_mask([4, 3], &alpha_mask, 8),
        Some(AlphaBounds {
            min_x: 1,
            min_y: 1,
            max_x: 3,
            max_y: 3,
        })
    );
}

#[test]
fn layout_trims_static_transparent_padding() {
    let layout = MascotWindowLayout::new(
        Vec2::new(100.0, 80.0),
        [10, 8],
        AlphaBounds {
            min_x: 2,
            min_y: 1,
            max_x: 8,
            max_y: 7,
        },
        zero_bounce(),
        zero_squash(),
        zero_idle_sink(),
    );

    assert_eq!(layout.window_size(), Vec2::new(72.0, 72.0));
    assert_eq!(layout.anchor_offset(), Vec2::new(36.0, 66.0));
    assert_eq!(layout.shake_amplitude_px(), 6.0);
    assert_eq!(
        layout.image_rect(Vec2::new(100.0, 80.0), MotionTransform::identity()),
        Rect::from_min_max(Pos2::new(-14.0, -4.0), Pos2::new(86.0, 76.0))
    );
}

#[test]
fn layout_reserves_room_for_motion_extrema() {
    let layout = MascotWindowLayout::new(
        Vec2::new(100.0, 100.0),
        [10, 10],
        AlphaBounds {
            min_x: 2,
            min_y: 2,
            max_x: 8,
            max_y: 8,
        },
        BounceAnimationConfig {
            amplitude_px: 12.0,
            ..Default::default()
        },
        SquashBounceAnimationConfig {
            amplitude_px: 0.0,
            squash_amount: 0.2,
            stretch_amount: 0.0,
            ..Default::default()
        },
        IdleSinkAnimationConfig {
            sink_amount: 0.08,
            lift_amount: 0.03,
            ..Default::default()
        },
    );

    assert!((layout.window_size().x - 74.72).abs() < 0.001);
    assert!((layout.window_size().y - 79.36).abs() < 0.001);
}

#[test]
fn anchored_inner_origin_preserves_bottom_center_anchor_across_layouts() {
    let previous_layout = MascotWindowLayout::new(
        Vec2::new(100.0, 80.0),
        [10, 8],
        AlphaBounds {
            min_x: 2,
            min_y: 1,
            max_x: 8,
            max_y: 7,
        },
        zero_bounce(),
        zero_squash(),
        zero_idle_sink(),
    );
    let next_layout = MascotWindowLayout::new(
        Vec2::new(100.0, 80.0),
        [10, 8],
        AlphaBounds {
            min_x: 1,
            min_y: 0,
            max_x: 9,
            max_y: 8,
        },
        zero_bounce(),
        zero_squash(),
        zero_idle_sink(),
    );

    let next_origin = anchored_inner_origin(Pos2::new(400.0, 300.0), previous_layout, next_layout);

    let previous_anchor = Pos2::new(400.0, 300.0) + previous_layout.anchor_offset();
    let next_anchor = next_origin + next_layout.anchor_offset();
    assert_eq!(previous_anchor, next_anchor);
}
