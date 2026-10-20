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
        ensemble_scale: Some(0.8),
        zip_path: workspace_path("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: Some(workspace_cache_root().join("demo/display-diffs/basic.json")),
    };

    write_mascot_config(&config_path, &target).expect("should write mascot runtime state");
    let loaded = load_mascot_config(&config_path).expect("should read mascot config");

    assert_eq!(loaded.png_path, target.png_path);
    assert_eq!(loaded.scale, target.scale);
    assert_eq!(loaded.ensemble_scale, target.ensemble_scale);
    assert_eq!(loaded.zip_path, target.zip_path);
    assert_eq!(loaded.psd_path_in_zip, target.psd_path_in_zip);
    assert_eq!(loaded.display_diff_path, target.display_diff_path);
    assert!(loaded.ui_font_paths.is_empty());
    assert!(loaded.always_idle_sink_enabled);
    assert_eq!(loaded.always_bend, AlwaysBendConfig::default());
    assert!(loaded.always_bend.enabled);
    assert_eq!(loaded.ensemble_mode, MascotEnsembleMode::SingleCharacter);
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
    assert!(!static_toml.contains("ensemble_scale ="));
    assert!(!static_toml.contains("transparent_background_click_through ="));
    assert!(!static_toml.contains("flash_blue_background_on_transparent_input ="));
    assert!(static_toml.contains("always_idle_sink = true"));
    assert!(static_toml.contains("always_bend = true"));
    assert!(static_toml.contains("[bend]"));
    let bend_table = extract_bend_table(&static_toml);
    assert!(bend_table
        .get("amplitude_ratio")
        .and_then(toml::Value::as_float)
        .is_some_and(|value| (value - 0.0075).abs() < 1e-6));
    assert!(static_toml.contains("[idle_sink]"));
    assert!(!static_toml.contains("[head_hitbox]"));
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
fn load_mascot_config_reads_current_static_settings_without_removed_keys() {
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
"#,
    )
    .expect("should seed mascot config without removed settings");
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

    assert!(loaded.ui_font_paths.is_empty());
    assert!(loaded.always_idle_sink_enabled);
    assert!(loaded.always_bend.enabled);
    assert_eq!(
        loaded.always_bend.amplitude_ratio,
        AlwaysBendConfig::default().amplitude_ratio
    );
    assert_eq!(loaded.ensemble_mode, MascotEnsembleMode::SingleCharacter);
    assert_eq!(
        loaded.always_idle_sink,
        IdleSinkAnimationConfig::default_for_always_bouncing()
    );
    assert!(runtime_state_path.exists());
}

#[test]
fn load_mascot_config_defaults_to_enabled_idle_sink_and_bend() {
    let config_path =
        workspace_cache_root().join("test-mascot-default-enabled/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-default-enabled"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-default-enabled"))
        .expect("should create temp directory");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/default/render.png",
  "zip_path": "assets/zip/default.zip",
  "psd_path_in_zip": "default/basic.psd",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");

    let loaded = load_mascot_config(&config_path).expect("config should load from defaults");

    assert!(loaded.ui_font_paths.is_empty());
    assert!(loaded.always_idle_sink_enabled);
    assert!(loaded.always_bend.enabled);
    assert_eq!(
        loaded.always_bend.amplitude_ratio,
        AlwaysBendConfig::default().amplitude_ratio
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
always_idle_sink = true
always_bend = true
ensemble_mode = "favorite"
ui_font_paths = ["fonts/custom-ui.ttf", "C:\\Windows\\Fonts\\meiryo.ttc"]

[bend]
amplitude_ratio = 0.02

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
        ensemble_scale: Some(0.95),
        zip_path: workspace_path("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: None,
    };
    write_mascot_config(&config_path, &target).expect("should update mascot runtime state");

    let loaded =
        load_mascot_config(&config_path).expect("should read split mascot config/state files");
    assert_eq!(loaded.png_path, target.png_path);
    assert_eq!(loaded.scale, target.scale);
    assert_eq!(loaded.ensemble_scale, target.ensemble_scale);
    assert_eq!(
        loaded.ui_font_paths,
        vec![
            workspace_cache_root()
                .join("test-mascot-preserve-current")
                .join("fonts/custom-ui.ttf"),
            PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
        ]
    );
    assert!(loaded.always_idle_sink_enabled);
    assert_eq!(
        loaded.always_bend,
        AlwaysBendConfig {
            enabled: true,
            amplitude_ratio: 0.02,
        }
    );
    assert_eq!(loaded.ensemble_mode, MascotEnsembleMode::Favorite);
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
    assert!(static_toml.contains("ensemble_mode = \"favorite\""));
    let static_value =
        toml::from_str::<toml::Value>(&static_toml).expect("static TOML should parse");
    let ui_font_paths = static_value
        .get("ui_font_paths")
        .and_then(toml::Value::as_array)
        .cloned()
        .expect("static TOML should preserve ui_font_paths");
    assert_eq!(
        ui_font_paths,
        vec![
            toml::Value::String("fonts/custom-ui.ttf".to_string()),
            toml::Value::String(r"C:\Windows\Fonts\meiryo.ttc".to_string()),
        ]
    );
    assert!(!static_toml.contains("transparent_background_click_through ="));
    assert!(!static_toml.contains("flash_blue_background_on_transparent_input ="));
    assert!(!static_toml.contains("[idle_sink]"));
    assert!(!static_toml.contains("[head_hitbox]"));
    let runtime_json =
        fs::read_to_string(&runtime_state_path).expect("should write mascot runtime JSON");
    assert!(runtime_json.contains("\"png_path\""));
    assert!(runtime_json.contains("\"ensemble_scale\": 0.95"));
    assert!(runtime_json.contains("\"demo/basic.psd\""));
}

#[test]
fn load_mascot_config_keeps_windows_absolute_ui_font_paths() {
    let config_path =
        workspace_cache_root().join("test-mascot-windows-fonts/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-windows-fonts"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-windows-fonts"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
ui_font_paths = [
  "fonts/local-ui.ttf",
  "C:\\Windows\\Fonts\\meiryo.ttc",
  "D:/Fonts/custom-ui.ttf",
  "\\\\font-server\\share\\ui.ttf",
]
"#,
    )
    .expect("should seed mascot static config");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "png_path": "cache/default/render.png",
  "zip_path": "assets/zip/default.zip",
  "psd_path_in_zip": "default/basic.psd",
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state");

    let loaded = load_mascot_config(&config_path).expect("config should load");

    assert_eq!(
        loaded.ui_font_paths,
        vec![
            workspace_cache_root()
                .join("test-mascot-windows-fonts")
                .join("fonts/local-ui.ttf"),
            PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
            PathBuf::from("D:/Fonts/custom-ui.ttf"),
            PathBuf::from(r"\\font-server\share\ui.ttf"),
        ]
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

#[test]
fn playback_motion_parameters_reload_independently_and_survive_target_writes() {
    let root = workspace_cache_root().join("test-playback-motion-config");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("mascot.toml");
    let target = MascotTarget {
        png_path: root.join("render.png"),
        scale: Some(0.45),
        ensemble_scale: None,
        zip_path: root.join("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        display_diff_path: None,
    };
    write_mascot_config(&path, &target).unwrap();
    let original = load_mascot_config(&path).unwrap();
    assert_eq!(original.musical_squash_bounce.duration_ms, 500);
    assert_eq!(original.musical_squash_bounce.amplitude_px, 3.0);
    assert_eq!(original.musical_squash_bounce.cycles, 1.0);
    assert_eq!(original.musical_squash_bounce.damping, 2.0);
    assert_eq!(original.musical_squash_bounce.squash_amount, 0.1);
    assert_eq!(original.musical_squash_bounce.stretch_amount, 0.05);
    let mut toml: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let playback = toml
        .get_mut("musical_squash_bounce")
        .unwrap()
        .as_table_mut()
        .unwrap();
    playback.insert("duration_ms".into(), 920.into());
    playback.insert("amplitude_px".into(), 61.0.into());
    playback.insert("cycles".into(), 2.3.into());
    playback.insert("damping".into(), 1.1.into());
    playback.insert("squash_amount".into(), 0.25.into());
    playback.insert("stretch_amount".into(), 0.03.into());
    fs::write(&path, toml::to_string_pretty(&toml).unwrap()).unwrap();
    let reloaded = load_mascot_config(&path).unwrap();
    let motion = reloaded.musical_squash_bounce;
    assert_eq!(motion.duration_ms, 920);
    assert_eq!(motion.amplitude_px, 61.0);
    assert_eq!(motion.cycles, 2.3);
    assert_eq!(motion.damping, 1.1);
    assert_eq!(motion.squash_amount, 0.25);
    assert_eq!(motion.stretch_amount, 0.03);
    assert_eq!(reloaded.squash_bounce, original.squash_bounce);
    assert_eq!(reloaded.bounce, original.bounce);
    let bounds = reloaded.squash_motion_bounds();
    assert_eq!(bounds.amplitude_px, 61.0);
    assert_eq!(bounds.squash_amount, 0.25);
    assert_eq!(bounds.stretch_amount, original.squash_bounce.stretch_amount);
    write_mascot_config(&path, &target).unwrap();
    assert_eq!(
        load_mascot_config(&path).unwrap().musical_squash_bounce,
        motion
    );
}

#[test]
#[ignore = "requires REAL_MUSICAL_CONFIG_PATH and REAL_MUSICAL_STATE_PATH; reads real TOML with a runtime snapshot"]
fn real_musical_config_loads_without_changing_existing_settings() {
    let path = PathBuf::from(std::env::var_os("REAL_MUSICAL_CONFIG_PATH").unwrap());
    let state = PathBuf::from(std::env::var_os("REAL_MUSICAL_STATE_PATH").unwrap());
    let before = fs::read(&path).unwrap();
    let runtime_bytes = fs::read(&state).unwrap();
    let snapshot_path = mascot_runtime_state_path(&path);
    fs::create_dir_all(snapshot_path.parent().unwrap()).unwrap();
    fs::write(&snapshot_path, &runtime_bytes).unwrap();
    let raw: toml::Value = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
    let expected: crate::SquashBounceAnimationConfig =
        raw["musical_squash_bounce"].clone().try_into().unwrap();
    let click: crate::SquashBounceAnimationConfig =
        raw["squash_bounce"].clone().try_into().unwrap();
    for _ in 0..2 {
        let loaded = load_mascot_config(&path).unwrap();
        assert_eq!(loaded.musical_squash_bounce, expected);
        assert_eq!(loaded.squash_bounce, click);
        let pose = loaded
            .musical_squash_bounce
            .sample(std::time::Duration::ZERO);
        assert!((pose.scale_y - (1.0 - expected.stretch_amount)).abs() < 0.00001);
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&state).unwrap(), runtime_bytes);
}
