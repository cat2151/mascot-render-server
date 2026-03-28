use super::*;

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
