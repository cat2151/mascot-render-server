use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use crate::{
    default_mascot_scale_for_screen_height, load_mascot_config, mascot_config_path,
    mascot_runtime_state_path, mascot_window_size, parse_mascot_config_path,
    psd_viewer_tui_activity_path, workspace_cache_root, workspace_path, write_mascot_config,
    BounceAlgorithm, HeadHitbox, IdleSinkAnimationConfig, MascotTarget, SquashAlgorithm,
};

#[test]
fn mascot_cli_defaults_to_root_config_path() {
    let path = parse_mascot_config_path([OsString::from("mascot-render-server")])
        .expect("should use default mascot-render-server.toml path");

    assert_eq!(path, mascot_config_path());
}

#[test]
fn mascot_cli_accepts_config_flag() {
    let path = parse_mascot_config_path([
        OsString::from("mascot-render-server"),
        OsString::from("--config"),
        OsString::from("demo-mascot-render-server.toml"),
    ])
    .expect("should parse --config path");

    assert_eq!(path, PathBuf::from("demo-mascot-render-server.toml"));
}

#[test]
fn mascot_runtime_state_path_is_derived_from_config_path() {
    let default_state = mascot_runtime_state_path(&PathBuf::from("mascot-render-server.toml"));
    let custom_state = mascot_runtime_state_path(&PathBuf::from("configs/demo.toml"));
    let default_activity =
        psd_viewer_tui_activity_path(&PathBuf::from("mascot-render-server.toml"));
    let custom_activity = psd_viewer_tui_activity_path(&PathBuf::from("configs/demo.toml"));

    assert!(default_state.starts_with(workspace_cache_root()));
    assert!(custom_state.starts_with(workspace_cache_root()));
    assert_ne!(default_state, custom_state);
    assert!(default_activity.starts_with(workspace_cache_root()));
    assert!(custom_activity.starts_with(workspace_cache_root()));
    assert_ne!(default_activity, custom_activity);
    assert!(custom_state
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with("demo-") && value.ends_with(".state.json")));
    assert!(custom_activity
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(
            |value| value.starts_with("demo-") && value.ends_with(".psd-viewer-tui-active")
        ));
}

#[test]
fn mascot_config_round_trips_through_static_toml_and_runtime_json() {
    let config_path = workspace_cache_root().join("test-mascot/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot"));
    let _ = fs::remove_file(&runtime_state_path);

    let target = MascotTarget {
        png_path: workspace_cache_root().join("demo/render.png"),
        scale: Some(0.35),
        favorite_ensemble_scale: Some(0.8),
        zip_path: workspace_path("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: Some(workspace_cache_root().join("demo/display-diffs/basic.json")),
    };

    write_mascot_config(&config_path, &target).expect("should write mascot runtime state");
    let loaded = load_mascot_config(&config_path).expect("should read mascot config");

    assert_eq!(loaded.png_path, target.png_path);
    assert_eq!(loaded.scale, target.scale);
    assert_eq!(
        loaded.favorite_ensemble_scale,
        target.favorite_ensemble_scale
    );
    assert_eq!(loaded.zip_path, target.zip_path);
    assert_eq!(loaded.psd_path_in_zip, target.psd_path_in_zip);
    assert_eq!(loaded.display_diff_path, target.display_diff_path);
    assert!(!loaded.always_bouncing);
    assert!(!loaded.always_bend);
    assert!(!loaded.favorite_ensemble_enabled);
    assert!(!loaded.transparent_background_click_through);
    assert!(loaded.flash_blue_background_on_transparent_input);
    assert_eq!(loaded.head_hitbox, HeadHitbox::default());
    assert_eq!(loaded.bounce.algorithm, BounceAlgorithm::DampedSine);
    assert_eq!(
        loaded.squash_bounce.algorithm,
        SquashAlgorithm::SquashStretch
    );
    assert_eq!(
        loaded.always_idle_sink,
        IdleSinkAnimationConfig::default_for_always_bouncing()
    );

    let static_toml =
        fs::read_to_string(&config_path).expect("should write mascot static config TOML");
    assert!(!static_toml.contains("png_path ="));
    assert!(!static_toml.contains("zip_path ="));
    assert!(!static_toml.contains("version ="));
    assert!(!static_toml.contains("updated_at ="));
    assert!(!static_toml.contains("favorite_ensemble_scale ="));
    assert!(static_toml.contains("flash_blue_background_on_transparent_input = true"));
    assert!(static_toml.contains("[always_idle_sink]"));
    assert!(
        runtime_state_path.exists(),
        "runtime state should be written"
    );
}

#[test]
fn load_mascot_config_defaults_flash_blue_background_when_key_is_missing() {
    let config_path =
        workspace_cache_root().join("test-mascot-default-flash/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-default-flash"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-default-flash"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
always_bouncing = true
always_bend = true
transparent_background_click_through = false
"#,
    )
    .expect("should seed mascot config without flash setting");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/legacy/render.png",
  "zip_path": "assets/zip/legacy.zip",
  "psd_path_in_zip": "legacy/basic.psd",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");

    let loaded = load_mascot_config(&config_path).expect("config should load");

    assert!(loaded.always_bouncing);
    assert!(loaded.always_bend);
    assert!(!loaded.favorite_ensemble_enabled);
    assert!(loaded.flash_blue_background_on_transparent_input);
    assert_eq!(
        loaded.always_idle_sink,
        IdleSinkAnimationConfig::default_for_always_bouncing()
    );
    assert!(runtime_state_path.exists());
}

#[test]
fn load_mascot_config_rejects_legacy_debug_flash_key() {
    let config_path =
        workspace_cache_root().join("test-mascot-legacy-flash/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-legacy-flash"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-legacy-flash"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
always_bouncing = true
always_bend = true
transparent_background_click_through = true
debug_flash_blue_background_on_transparent_input = true

[head_hitbox]
x = 0.3
y = 0.1
width = 0.2
height = 0.2

[bounce]
algorithm = "damped_sine"
duration_ms = 1200
amplitude_px = 22.0
cycles = 1.2
damping = 1.8

[squash_bounce]
algorithm = "squash_stretch"
duration_ms = 640
amplitude_px = 16.0
cycles = 1.1
damping = 1.5
squash_amount = 0.22
stretch_amount = 0.08
"#,
    )
    .expect("should seed mascot config with legacy debug key");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/legacy/render.png",
  "scale": 0.42,
  "favorite_ensemble_scale": 0.9,
  "zip_path": "assets/zip/legacy.zip",
  "psd_path_in_zip": "legacy/basic.psd",
  "display_diff_path": "cache/legacy/basic.json",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");

    let error = load_mascot_config(&config_path).expect_err("legacy debug key should be rejected");
    assert!(
        format!("{error:#}").contains("debug_flash_blue_background_on_transparent_input"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn load_mascot_config_rejects_legacy_always_squash_bounce_section() {
    let config_path =
        workspace_cache_root().join("test-mascot-legacy-idle-name/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-legacy-idle-name"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-legacy-idle-name"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
[always_squash_bounce]
duration_ms = 1200
amplitude_px = 9.0
squash_amount = 0.08
stretch_amount = 0.05
"#,
    )
    .expect("should seed legacy idle config");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/legacy/render.png",
  "zip_path": "assets/zip/legacy.zip",
  "psd_path_in_zip": "legacy/basic.psd",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");

    let error =
        load_mascot_config(&config_path).expect_err("legacy idle section should be rejected");

    assert!(
        format!("{error:#}").contains("always_squash_bounce"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn writing_mascot_config_preserves_current_static_sections() {
    let config_path =
        workspace_cache_root().join("test-mascot-preserve-current/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-preserve-current"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-preserve-current"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
always_bouncing = true
always_bend = true
favorite_ensemble_enabled = true
transparent_background_click_through = true
flash_blue_background_on_transparent_input = true

[head_hitbox]
x = 0.3
y = 0.1
width = 0.2
height = 0.2

[bounce]
algorithm = "damped_sine"
duration_ms = 1200
amplitude_px = 22.0
cycles = 1.2
damping = 1.8

[squash_bounce]
algorithm = "squash_stretch"
duration_ms = 640
amplitude_px = 16.0
cycles = 1.1
damping = 1.5
squash_amount = 0.22
stretch_amount = 0.08
"#,
    )
    .expect("should seed current mascot config");

    let target = MascotTarget {
        png_path: workspace_cache_root().join("demo/render.png"),
        scale: Some(0.45),
        favorite_ensemble_scale: Some(0.95),
        zip_path: workspace_path("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: None,
    };
    write_mascot_config(&config_path, &target).expect("should update mascot runtime state");

    let loaded =
        load_mascot_config(&config_path).expect("should read split mascot config/state files");
    assert_eq!(loaded.png_path, target.png_path);
    assert_eq!(loaded.scale, target.scale);
    assert_eq!(
        loaded.favorite_ensemble_scale,
        target.favorite_ensemble_scale
    );
    assert!(loaded.always_bouncing);
    assert!(loaded.always_bend);
    assert!(loaded.favorite_ensemble_enabled);
    assert!(loaded.transparent_background_click_through);
    assert!(loaded.flash_blue_background_on_transparent_input);
    assert_eq!(loaded.head_hitbox.x, 0.3);
    assert_eq!(loaded.bounce.duration_ms, 1200);
    assert_eq!(loaded.squash_bounce.squash_amount, 0.22);
    assert_eq!(
        loaded.always_idle_sink,
        IdleSinkAnimationConfig::default_for_always_bouncing()
    );

    let static_toml = fs::read_to_string(&config_path).expect("should keep mascot static config");
    assert!(!static_toml.contains("png_path ="));
    assert!(!static_toml.contains("psd_path_in_zip ="));
    assert!(!static_toml.contains("version ="));
    assert!(!static_toml.contains("updated_at ="));
    assert!(static_toml.contains("always_bouncing = true"));
    assert!(static_toml.contains("always_bend = true"));
    assert!(static_toml.contains("favorite_ensemble_enabled = true"));
    assert!(static_toml.contains("flash_blue_background_on_transparent_input = true"));
    let runtime_json =
        fs::read_to_string(&runtime_state_path).expect("should write mascot runtime JSON");
    assert!(runtime_json.contains("\"png_path\""));
    assert!(runtime_json.contains("\"favorite_ensemble_scale\": 0.95"));
    assert!(runtime_json.contains("\"demo/basic.psd\""));
}

#[test]
fn load_mascot_config_disables_favorite_ensemble_while_psd_viewer_tui_is_active() {
    let root = workspace_cache_root().join("test-mascot-active-psd-viewer-tui");
    let config_path = root.join("mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&runtime_state_path);
    let _ = fs::remove_file(&activity_path);

    fs::create_dir_all(&root).expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
favorite_ensemble_enabled = true
"#,
    )
    .expect("should seed mascot config");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/active/render.png",
  "zip_path": "assets/zip/active.zip",
  "psd_path_in_zip": "active/basic.psd",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");
    fs::write(&activity_path, crate::unix_timestamp().to_string())
        .expect("should write psd-viewer-tui heartbeat");

    let loaded = load_mascot_config(&config_path).expect("config should load");

    assert!(
        !loaded.favorite_ensemble_enabled,
        "psd-viewer-tui activity should temporarily disable favorite ensemble"
    );
}

#[test]
fn mascot_window_size_uses_scale_or_legacy_fallback() {
    assert_eq!(mascot_window_size(1200, 600, None), [480.0, 240.0]);
    assert_eq!(mascot_window_size(400, 200, Some(0.5)), [200.0, 100.0]);
}

#[test]
fn default_mascot_scale_targets_thirty_three_percent_of_screen_height() {
    let scale = default_mascot_scale_for_screen_height(1650, 1440);

    assert!((scale - 0.288).abs() < 0.001, "unexpected scale: {scale}");
}
