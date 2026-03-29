use std::time::Duration;

use eframe::egui::{pos2, Rect, TextureId};
use mascot_render_core::AlwaysBendConfig;

use crate::always_bend;
use crate::mascot_app::click_interaction_hit_test;

const LONG_RUNNING_PHASE_STABILITY_MS: u64 = 4_200 * 10_000; // About 11.7 hours of complete bend cycles.
const FLOAT_TOLERANCE: f32 = 1e-6;

#[test]
fn sample_always_bend_swings_left_and_right() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 480.0));
    let config = AlwaysBendConfig::default();

    let resting = always_bend::sample_always_bend(Duration::ZERO, image_rect, config);
    let right = always_bend::sample_always_bend(Duration::from_millis(1_050), image_rect, config);
    let left = always_bend::sample_always_bend(Duration::from_millis(3_150), image_rect, config);

    assert_eq!(resting.offset_x, 0.0);
    assert!(right.offset_x > 0.0);
    assert!(left.offset_x < 0.0);
    assert!(right.offset_x.abs() <= image_rect.width() * config.amplitude_ratio);
}

#[test]
fn sample_always_bend_uses_configured_amplitude_ratio() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 480.0));
    let config = AlwaysBendConfig {
        enabled: true,
        amplitude_ratio: 0.05,
    };
    let bend = always_bend::sample_always_bend(Duration::from_millis(1_050), image_rect, config);
    let expected_offset = image_rect.width() * config.amplitude_ratio;

    assert!((bend.offset_x - expected_offset).abs() <= FLOAT_TOLERANCE);
}

#[test]
fn always_bend_mesh_moves_upper_center_more_than_lower_center() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(120.0, 240.0));
    let bend = always_bend::sample_always_bend(
        Duration::from_millis(1_050),
        image_rect,
        AlwaysBendConfig::default(),
    );
    let mesh = always_bend::mesh(TextureId::Managed(7), image_rect, bend);

    let stride = 5;
    let top_center = mesh.vertices[2].pos.x;
    let middle_center = mesh.vertices[(6 * stride) + 2].pos.x;
    let bottom_center = mesh.vertices[(12 * stride) + 2].pos.x;
    let edge_x = mesh.vertices[0].pos.x;

    assert!(top_center > middle_center);
    assert!(middle_center > bottom_center);
    assert_eq!(bottom_center, image_rect.center().x);
    assert_eq!(edge_x, image_rect.left());
}

#[test]
fn sample_always_bend_preserves_phase_after_many_cycles() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 480.0));
    let config = AlwaysBendConfig::default();
    let short = always_bend::sample_always_bend(Duration::from_millis(1_050), image_rect, config);
    let long = always_bend::sample_always_bend(
        Duration::from_millis(LONG_RUNNING_PHASE_STABILITY_MS) + Duration::from_millis(1_050),
        image_rect,
        config,
    );

    assert!((short.offset_x - long.offset_x).abs() <= FLOAT_TOLERANCE);
    assert_eq!(short.offset_y, long.offset_y);
    assert_eq!(short.scale_x, long.scale_x);
    assert_eq!(short.scale_y, long.scale_y);
    assert_eq!(short.offset_y, 0.0);
    assert_eq!(short.scale_x, 1.0);
    assert_eq!(short.scale_y, 1.0);
}

#[test]
fn click_interaction_uses_full_image_rect() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 480.0));
    let body_point = pos2(20.0, 430.0);
    let outside_point = pos2(-1.0, 430.0);

    assert!(click_interaction_hit_test(image_rect, body_point));
    assert!(!click_interaction_hit_test(image_rect, outside_point));
}
