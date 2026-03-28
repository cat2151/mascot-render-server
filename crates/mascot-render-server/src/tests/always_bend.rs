use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{pos2, Rect, TextureId};
use mascot_render_core::{
    AlwaysBendConfig, BounceAnimationConfig, HeadHitbox, IdleSinkAnimationConfig, MascotConfig,
    SquashBounceAnimationConfig,
};

use crate::always_bend;
use crate::mascot_app::{allows_precise_pointer_interaction, transparent_hit_test_enabled};

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

    assert_eq!(bend.offset_x, image_rect.width() * config.amplitude_ratio);
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
fn always_bend_disables_precise_pointer_hit_testing() {
    let mut config = sample_config();
    config.transparent_background_click_through = true;

    assert!(transparent_hit_test_enabled(&config));
    assert!(allows_precise_pointer_interaction(&config));

    let bent_config = MascotConfig {
        always_bend: AlwaysBendConfig {
            enabled: true,
            ..config.always_bend
        },
        ..config
    };
    assert!(!transparent_hit_test_enabled(&bent_config));
    assert!(!allows_precise_pointer_interaction(&bent_config));
}

fn sample_config() -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("cache/demo/render.png"),
        scale: Some(1.0),
        favorite_ensemble_scale: None,
        zip_path: PathBuf::from("assets/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: None,
        always_idle_sink_enabled: false,
        always_bend: AlwaysBendConfig::default(),
        favorite_ensemble_enabled: false,
        transparent_background_click_through: false,
        flash_blue_background_on_transparent_input: true,
        head_hitbox: HeadHitbox::default(),
        bounce: BounceAnimationConfig::default(),
        squash_bounce: SquashBounceAnimationConfig::default(),
        always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
    }
}
