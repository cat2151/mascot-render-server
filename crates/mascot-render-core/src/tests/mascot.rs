use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use crate::{
    default_mascot_scale_for_screen_height, load_mascot_config, mascot_config_path,
    mascot_runtime_state_path, mascot_window_size, parse_mascot_config_path,
    psd_viewer_tui_activity_path, workspace_cache_root, workspace_path, write_mascot_config,
    AlwaysBendConfig, BounceAlgorithm, HeadHitbox, IdleSinkAnimationConfig, MascotTarget,
    SquashAlgorithm,
};

mod cli_and_paths;
mod config_roundtrip;
mod favorite_ensemble;
mod legacy_rejections;

fn extract_idle_sink_table(static_toml: &str) -> toml::value::Table {
    toml::from_str::<toml::Value>(static_toml)
        .expect("static TOML should parse")
        .get("idle_sink")
        .and_then(toml::Value::as_table)
        .cloned()
        .expect("static TOML should contain idle_sink section")
}

fn seed_favorite_ensemble_config(test_name: &str) -> (PathBuf, PathBuf) {
    let root = workspace_cache_root().join(test_name);
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

    (config_path, activity_path)
}
