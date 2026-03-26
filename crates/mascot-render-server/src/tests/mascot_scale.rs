use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::Modifiers;
use mascot_render_core::{
    load_mascot_config, mascot_runtime_state_path, workspace_cache_root, MascotConfig,
    SquashBounceAnimationConfig,
};

use crate::mascot_scale::{
    adjust_scale, effective_scale, keyboard_scale_steps, persist_scale, scroll_scale_steps,
};

fn unique_test_config_path(test_name: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    workspace_cache_root().join(format!(
        "test-mascot-scale-{test_name}-{unique_suffix}/mascot-render-server.toml"
    ))
}

fn sample_config() -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("cache/demo/render.png"),
        scale: None,
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: Some(PathBuf::from("cache/demo/variation.json")),
        always_bouncing: false,
        transparent_background_click_through: false,
        flash_blue_background_on_transparent_input: true,
        head_hitbox: Default::default(),
        bounce: Default::default(),
        squash_bounce: Default::default(),
        always_squash_bounce: SquashBounceAnimationConfig::default_for_always_bouncing(),
    }
}

fn assert_close(actual: f32, expected: f32) {
    const FLOAT_TOLERANCE: f32 = 0.001;
    assert!(
        (actual - expected).abs() <= FLOAT_TOLERANCE,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn effective_scale_uses_legacy_window_size_when_scale_is_missing() {
    assert_close(effective_scale(960, 480, None), 0.5);
    assert_close(effective_scale(120, 80, None), 1.0);
    assert_close(effective_scale(120, 80, Some(0.42)), 0.42);
}

#[test]
fn adjust_scale_uses_ten_percent_steps_and_clamps_to_minimum() {
    assert_close(adjust_scale(0.35, 1).expect("scale should increase"), 0.45);
    assert_close(adjust_scale(0.35, -1).expect("scale should decrease"), 0.25);
    assert_close(
        adjust_scale(0.05, -1).expect("scale should clamp to minimum"),
        0.01,
    );
    assert_eq!(adjust_scale(0.01, -1), None);
    assert_eq!(adjust_scale(0.35, 0), None);
}

#[test]
fn keyboard_and_scroll_inputs_map_to_single_scale_steps() {
    assert_eq!(
        keyboard_scale_steps(Modifiers::NONE, true, false),
        1,
        "plain plus/equals should increase the scale"
    );
    assert_eq!(
        keyboard_scale_steps(Modifiers::SHIFT, true, false),
        1,
        "shift should still allow plus on the main keyboard"
    );
    assert_eq!(
        keyboard_scale_steps(Modifiers::CTRL, true, false),
        0,
        "command modifiers should not trigger resizing"
    );
    assert_eq!(keyboard_scale_steps(Modifiers::NONE, false, true), -1);
    assert_eq!(scroll_scale_steps(14.0), 1);
    assert_eq!(scroll_scale_steps(-14.0), -1);
    assert_eq!(scroll_scale_steps(0.0), 0);
}

#[test]
fn persist_scale_updates_runtime_state() {
    let config_path = unique_test_config_path("persist");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);

    let config = sample_config();
    persist_scale(&config_path, &config, 0.42).expect("should persist runtime state scale");
    let loaded = load_mascot_config(&config_path).expect("persisted config should load");

    assert_eq!(loaded.scale, Some(0.42));
    assert_eq!(loaded.png_path, config.png_path);
    assert_eq!(loaded.zip_path, config.zip_path);
    assert_eq!(loaded.psd_path_in_zip, config.psd_path_in_zip);
    assert_eq!(loaded.display_diff_path, config.display_diff_path);

    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);
}
