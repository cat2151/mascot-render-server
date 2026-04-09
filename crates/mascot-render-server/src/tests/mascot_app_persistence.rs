use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use mascot_render_core::{
    load_mascot_config, mascot_runtime_state_path, workspace_cache_root, AlwaysBendConfig,
    IdleSinkAnimationConfig, MascotConfig,
};

use crate::mascot_app::{
    persist_requested_skin_change_for_test, verify_persisted_skin_change_for_test,
};

fn unique_test_config_path(test_name: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    workspace_cache_root().join(format!(
        "test-mascot-app-persistence-{test_name}-{unique_suffix}/mascot-render-server.toml"
    ))
}

fn sample_config() -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("cache/demo/render.png"),
        scale: Some(0.42),
        favorite_ensemble_scale: Some(0.8),
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        display_diff_path: Some(PathBuf::from("cache/demo/variation.json")),
        always_idle_sink_enabled: false,
        always_bend: AlwaysBendConfig::default(),
        favorite_ensemble_enabled: false,
        bounce: Default::default(),
        squash_bounce: Default::default(),
        always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
    }
}

#[test]
fn persist_requested_skin_change_updates_runtime_state_png_path() {
    let config_path = unique_test_config_path("persist");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);

    let config = sample_config();
    let requested_png_path = PathBuf::from("cache/demo/other.png");
    persist_requested_skin_change_for_test(&config_path, &config, &requested_png_path)
        .expect("should persist requested skin change");
    let loaded = load_mascot_config(&config_path).expect("persisted config should load");

    assert_eq!(loaded.png_path, requested_png_path);
    assert_eq!(loaded.scale, config.scale);
    assert_eq!(
        loaded.favorite_ensemble_scale,
        config.favorite_ensemble_scale
    );
    assert_eq!(loaded.zip_path, config.zip_path);
    assert_eq!(loaded.psd_path_in_zip, config.psd_path_in_zip);
    assert_eq!(loaded.display_diff_path, config.display_diff_path);

    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);
}

#[test]
fn test_verify_persisted_skin_change_with_matching_path() {
    let config_path = unique_test_config_path("verify-match");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);

    let config = sample_config();
    let requested_png_path = PathBuf::from("cache/demo/requested.png");
    persist_requested_skin_change_for_test(&config_path, &config, &requested_png_path)
        .expect("should seed matching runtime state");

    let persisted_png_path =
        verify_persisted_skin_change_for_test(&config_path, &requested_png_path)
            .expect("matching persisted state should verify");
    assert_eq!(persisted_png_path, requested_png_path);

    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);
}

#[test]
fn verify_persisted_skin_change_reports_mismatch() {
    let config_path = unique_test_config_path("verify-mismatch");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);

    let config = sample_config();
    let persisted_png_path = PathBuf::from("cache/demo/persisted.png");
    let requested_png_path = PathBuf::from("cache/demo/requested.png");
    persist_requested_skin_change_for_test(&config_path, &config, &persisted_png_path)
        .expect("should seed runtime state");

    let error = verify_persisted_skin_change_for_test(&config_path, &requested_png_path)
        .expect_err("mismatch should be rejected");
    assert!(error
        .to_string()
        .contains("persisted mascot runtime state did not match the requested png_path"));
    assert!(error
        .to_string()
        .contains("requested=cache/demo/requested.png"));
    assert!(error
        .to_string()
        .contains("persisted=cache/demo/persisted.png"));

    if let Some(parent) = config_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    let _ = fs::remove_file(&runtime_state_path);
}
