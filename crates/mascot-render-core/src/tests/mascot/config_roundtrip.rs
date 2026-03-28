use super::*;

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
    assert!(!loaded.always_idle_sink_enabled);
    assert_eq!(loaded.always_bend, AlwaysBendConfig::default());
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
    assert!(static_toml.contains("always_bend = false"));
    assert!(static_toml.contains("[bend]"));
    assert!(static_toml.contains("[idle_sink]"));
    let idle_sink_table = extract_idle_sink_table(&static_toml);
    assert_eq!(
        idle_sink_table
            .get("algorithm")
            .and_then(toml::Value::as_str),
        Some("idle_sink")
    );
    assert_eq!(
        idle_sink_table
            .get("duration_ms")
            .and_then(toml::Value::as_integer),
        Some(2200)
    );
    assert!(idle_sink_table
        .get("sink_amount")
        .and_then(toml::Value::as_float)
        .is_some_and(|value| (value - 0.0015).abs() < 1e-6));
    assert!(idle_sink_table
        .get("lift_amount")
        .and_then(toml::Value::as_float)
        .is_some_and(|value| (value - 0.0015).abs() < 1e-6));
    assert!(!idle_sink_table.contains_key("amplitude_px"));
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
always_idle_sink = true
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

    assert!(loaded.always_idle_sink_enabled);
    assert!(loaded.always_bend.enabled);
    assert_eq!(
        loaded.always_bend.amplitude_ratio,
        AlwaysBendConfig::default().amplitude_ratio
    );
    assert!(!loaded.favorite_ensemble_enabled);
    assert!(loaded.flash_blue_background_on_transparent_input);
    assert_eq!(
        loaded.always_idle_sink,
        IdleSinkAnimationConfig::default_for_always_bouncing()
    );
    assert!(runtime_state_path.exists());
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
always_idle_sink = true
always_bend = true
favorite_ensemble_enabled = true
transparent_background_click_through = true
flash_blue_background_on_transparent_input = true

[bend]
amplitude_ratio = 0.02

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
    assert!(loaded.always_idle_sink_enabled);
    assert_eq!(
        loaded.always_bend,
        AlwaysBendConfig {
            enabled: true,
            amplitude_ratio: 0.02,
        }
    );
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
    assert!(static_toml.contains("always_idle_sink = true"));
    assert!(static_toml.contains("always_bend = true"));
    assert!(static_toml.contains("[bend]"));
    assert!(static_toml.contains("amplitude_ratio = 0.02"));
    assert!(static_toml.contains("favorite_ensemble_enabled = true"));
    assert!(static_toml.contains("flash_blue_background_on_transparent_input = true"));
    assert!(!static_toml.contains("[idle_sink]"));
    let runtime_json =
        fs::read_to_string(&runtime_state_path).expect("should write mascot runtime JSON");
    assert!(runtime_json.contains("\"png_path\""));
    assert!(runtime_json.contains("\"favorite_ensemble_scale\": 0.95"));
    assert!(runtime_json.contains("\"demo/basic.psd\""));
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
