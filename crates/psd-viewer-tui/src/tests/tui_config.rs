use std::fs;
use std::path::PathBuf;

use crate::tui_config::{
    load_tui_config, load_tui_runtime_state, save_tui_config, save_tui_runtime_state,
    tui_runtime_state_path, PsdRuntimeState, TuiConfig, TuiRuntimeState,
    DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
};
use mascot_render_core::workspace_cache_root;
use mascot_render_core::EyeBlinkTarget;

#[test]
fn default_layer_scroll_margin_ratio_is_quarter_height() {
    assert_eq!(
        TuiConfig::default().layer_scroll_margin_ratio,
        DEFAULT_LAYER_SCROLL_MARGIN_RATIO
    );
    assert_eq!(DEFAULT_LAYER_SCROLL_MARGIN_RATIO, 0.25);
}

#[test]
fn tui_config_round_trips_static_settings() {
    let path = workspace_cache_root().join("test-tui-config/psd-viewer-tui.toml");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-config"));

    save_tui_config(
        &path,
        &TuiConfig {
            layer_scroll_margin_ratio: 0.33,
            eye_blink_targets: vec![EyeBlinkTarget {
                psd_file_name: "blink.psd".to_string(),
                first_layer_name: "open".to_string(),
                second_layer_name: "closed".to_string(),
            }],
        },
    )
    .expect("should write TUI config");

    let loaded = load_tui_config(&path).expect("should read TUI config");
    assert_eq!(
        loaded,
        TuiConfig {
            layer_scroll_margin_ratio: 0.33,
            eye_blink_targets: vec![EyeBlinkTarget {
                psd_file_name: "blink.psd".to_string(),
                first_layer_name: "open".to_string(),
                second_layer_name: "closed".to_string(),
            }],
        }
    );
}

#[test]
fn tui_runtime_state_round_trips_mascot_scale_per_psd() {
    let config_path = workspace_cache_root().join("test-tui-runtime/psd-viewer-tui.toml");
    let runtime_state_path = tui_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-runtime"));
    let _ = fs::remove_file(&runtime_state_path);

    save_tui_runtime_state(
        &config_path,
        &TuiRuntimeState {
            psd_states: vec![
                PsdRuntimeState {
                    zip_path: PathBuf::from("/workspace/a.zip"),
                    psd_path_in_zip: PathBuf::from("a/body.psd"),
                    mascot_scale: Some(0.37),
                },
                PsdRuntimeState {
                    zip_path: PathBuf::from("/workspace/b.zip"),
                    psd_path_in_zip: PathBuf::from("b/face.psd"),
                    mascot_scale: Some(0.91),
                },
            ],
        },
    )
    .expect("should write TUI runtime state");

    let loaded = load_tui_runtime_state(&config_path).expect("should read TUI runtime state JSON");
    assert_eq!(
        loaded,
        TuiRuntimeState {
            psd_states: vec![
                PsdRuntimeState {
                    zip_path: PathBuf::from("/workspace/a.zip"),
                    psd_path_in_zip: PathBuf::from("a/body.psd"),
                    mascot_scale: Some(0.37),
                },
                PsdRuntimeState {
                    zip_path: PathBuf::from("/workspace/b.zip"),
                    psd_path_in_zip: PathBuf::from("b/face.psd"),
                    mascot_scale: Some(0.91),
                },
            ],
        }
    );

    let raw = fs::read_to_string(runtime_state_path).expect("should read written runtime state");
    assert!(
        raw.contains("\"mascot_scale\""),
        "runtime state should keep per-PSD mascot_scale values"
    );
    assert!(
        !raw.contains("\"legacy_mascot_scale\""),
        "runtime state should not write removed compatibility fields"
    );
}

#[test]
fn invalid_tui_config_falls_back_to_default() {
    let path = workspace_cache_root().join("test-tui-config-invalid/psd-viewer-tui.toml");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-config-invalid"));
    fs::create_dir_all(workspace_cache_root().join("test-tui-config-invalid"))
        .expect("should create temp directory");
    fs::write(&path, "not = [valid").expect("should seed invalid TUI config");

    let loaded = load_tui_config(&path).expect("invalid config should fall back to default");
    assert_eq!(loaded, TuiConfig::default());
}

#[test]
fn invalid_tui_runtime_state_reports_error() {
    let config_path = workspace_cache_root().join("test-tui-runtime-invalid/psd-viewer-tui.toml");
    let runtime_state_path = tui_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-runtime-invalid"));
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(workspace_cache_root().join("test-tui-runtime-invalid"))
        .expect("should create temp directory");
    fs::write(&runtime_state_path, "{ invalid json").expect("should seed invalid runtime state");

    let error =
        load_tui_runtime_state(&config_path).expect_err("invalid runtime state should fail");
    assert!(
        format!("{error:#}").contains("failed to parse TUI runtime state"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn tui_runtime_state_with_unknown_field_reports_error() {
    let config_path =
        workspace_cache_root().join("test-tui-runtime-unknown-field/psd-viewer-tui.toml");
    let runtime_state_path = tui_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-runtime-unknown-field"));
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(workspace_cache_root().join("test-tui-runtime-unknown-field"))
        .expect("should create temp directory");
    fs::write(
        &runtime_state_path,
        r#"{
  "version": 1,
  "psd_states": [],
  "extra_field": true,
  "updated_at": 1
}"#,
    )
    .expect("should seed runtime state with unknown field");

    let error = load_tui_runtime_state(&config_path)
        .expect_err("runtime state with unknown field should fail");
    assert!(
        format!("{error:#}").contains("failed to parse TUI runtime state"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn tui_config_keeps_only_file_name_for_eye_blink_targets() {
    let path = workspace_cache_root().join("test-tui-config-filename/psd-viewer-tui.toml");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-config-filename"));

    save_tui_config(
        &path,
        &TuiConfig {
            layer_scroll_margin_ratio: 0.33,
            eye_blink_targets: vec![EyeBlinkTarget {
                psd_file_name: "nested/path/blink.psd".to_string(),
                first_layer_name: "open".to_string(),
                second_layer_name: "closed".to_string(),
            }],
        },
    )
    .expect("should write TUI config");

    let loaded = load_tui_config(&path).expect("should normalize eye blink target file names");
    assert_eq!(loaded.eye_blink_targets[0].psd_file_name, "blink.psd");
}

#[test]
fn configured_eye_blink_targets_are_preserved() {
    let path = workspace_cache_root().join("test-tui-config-eye-names/psd-viewer-tui.toml");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-config-eye-names"));
    fs::create_dir_all(workspace_cache_root().join("test-tui-config-eye-names"))
        .expect("should create temp directory");

    fs::write(
        &path,
        r#"
version = 1
layer_scroll_margin_ratio = 0.33

[[eye_blink_targets]]
psd_file_name = "ずんだもん立ち絵素材V3.2_基本版.psd"
first_layer_name = "普通目"
second_layer_name = "閉じ目"
"#,
    )
    .expect("should seed TUI config");

    let loaded = load_tui_config(&path).expect("should keep configured eye blink targets");
    assert_eq!(loaded.eye_blink_targets[0].first_layer_name, "普通目");
    assert_eq!(loaded.eye_blink_targets[0].second_layer_name, "閉じ目");
}

#[test]
fn missing_tui_runtime_state_defaults_to_empty_state() {
    let config_path = workspace_cache_root().join("test-tui-runtime-missing/psd-viewer-tui.toml");
    let runtime_state_path = tui_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-runtime-missing"));
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(workspace_cache_root().join("test-tui-runtime-missing"))
        .expect("should create temp directory");

    fs::write(
        &config_path,
        r#"
version = 1
layer_scroll_margin_ratio = 0.33

[[eye_blink_targets]]
psd_file_name = "blink.psd"
first_layer_name = "open"
second_layer_name = "closed"
"#,
    )
    .expect("should seed TUI config");

    let runtime_state =
        load_tui_runtime_state(&config_path).expect("missing runtime state should default");
    assert_eq!(runtime_state, TuiRuntimeState::default());
}

#[test]
fn legacy_combined_tui_config_falls_back_to_default_static_config() {
    let config_path = workspace_cache_root().join("test-tui-legacy-static/psd-viewer-tui.toml");
    let runtime_state_path = tui_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-tui-legacy-static"));
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(workspace_cache_root().join("test-tui-legacy-static"))
        .expect("should create temp directory");

    fs::write(
        &config_path,
        r#"
version = 1
mascot_scale = 0.41
layer_scroll_margin_ratio = 0.33
updated_at = 1774133654

[[eye_blink_targets]]
psd_file_name = "blink.psd"
first_layer_name = "open"
second_layer_name = "closed"
"#,
    )
    .expect("should seed legacy combined TUI config");

    let loaded = load_tui_config(&config_path).expect("legacy combined config should be ignored");
    assert_eq!(loaded, TuiConfig::default());
    assert!(
        !runtime_state_path.exists(),
        "runtime state should not be synthesized"
    );
}
