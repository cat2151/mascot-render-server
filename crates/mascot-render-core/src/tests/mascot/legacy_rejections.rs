use super::*;

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
always_idle_sink = true
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
fn load_mascot_config_rejects_legacy_always_bend_section() {
    let config_path =
        workspace_cache_root().join("test-mascot-legacy-always-bend/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-legacy-always-bend"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-legacy-always-bend"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
[always_bend]
enabled = true
"#,
    )
    .expect("should seed legacy always_bend section");
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

    let error = load_mascot_config(&config_path).expect_err("legacy section should be rejected");
    assert!(
        format!("{error:#}").contains("always_bend"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn load_mascot_config_rejects_legacy_always_bouncing_key() {
    let config_path =
        workspace_cache_root().join("test-mascot-legacy-always-bouncing/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-legacy-always-bouncing"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-legacy-always-bouncing"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
always_bouncing = true
"#,
    )
    .expect("should seed legacy always_bouncing config");
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

    let error = load_mascot_config(&config_path).expect_err("legacy key should be rejected");
    assert!(
        format!("{error:#}").contains("always_bouncing"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn load_mascot_config_rejects_legacy_always_idle_sink_section() {
    let config_path = workspace_cache_root()
        .join("test-mascot-legacy-always-idle-sink/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-legacy-always-idle-sink"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-legacy-always-idle-sink"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
[always_idle_sink]
duration_ms = 1200
amplitude_px = 9.0
sink_amount = 0.08
lift_amount = 0.05
"#,
    )
    .expect("should seed legacy always_idle_sink section");
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

    let error = load_mascot_config(&config_path).expect_err("legacy section should be rejected");
    assert!(
        format!("{error:#}").contains("always_idle_sink"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn load_mascot_config_rejects_idle_sink_amplitude_px() {
    let config_path =
        workspace_cache_root().join("test-mascot-idle-sink-amplitude/mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-mascot-idle-sink-amplitude"));
    let _ = fs::remove_file(&runtime_state_path);

    fs::create_dir_all(workspace_cache_root().join("test-mascot-idle-sink-amplitude"))
        .expect("should create temp directory");
    fs::write(
        &config_path,
        r#"
[idle_sink]
algorithm = "idle_sink"
duration_ms = 2200
amplitude_px = 0.0
sink_amount = 0.0015
lift_amount = 0.0015
"#,
    )
    .expect("should seed unsupported idle_sink amplitude config");
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
        load_mascot_config(&config_path).expect_err("idle_sink amplitude_px should be rejected");
    assert!(
        format!("{error:#}").contains("amplitude_px"),
        "unexpected error: {error:#}"
    );
}
