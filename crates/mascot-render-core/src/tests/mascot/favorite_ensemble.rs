use super::*;

#[test]
fn load_mascot_config_keeps_favorite_ensemble_enabled_while_psd_viewer_tui_is_active() {
    let (config_path, activity_path) =
        seed_favorite_ensemble_config("test-mascot-active-psd-viewer-tui");
    fs::write(&activity_path, crate::unix_timestamp().to_string())
        .expect("should write psd-viewer-tui heartbeat");

    let loaded = load_mascot_config(&config_path).expect("config should load");

    assert!(loaded.favorite_ensemble_enabled);
}

#[test]
fn favorite_ensemble_enabled_setting_can_be_toggled_in_static_config() {
    let (config_path, _activity_path) =
        seed_favorite_ensemble_config("test-mascot-toggle-favorite-ensemble");

    assert!(load_favorite_ensemble_enabled(&config_path).expect("config should load"));

    set_favorite_ensemble_enabled(&config_path, false).expect("should disable favorite ensemble");
    assert!(!load_favorite_ensemble_enabled(&config_path).expect("config should reload"));
    assert!(
        !load_mascot_config(&config_path)
            .expect("full config should reload")
            .favorite_ensemble_enabled
    );

    set_favorite_ensemble_enabled(&config_path, true).expect("should enable favorite ensemble");
    assert!(load_favorite_ensemble_enabled(&config_path).expect("config should reload"));
}

#[test]
fn load_mascot_config_ignores_psd_viewer_tui_heartbeat_when_favorite_ensemble_is_enabled() {
    let cases = vec![
        (String::new(), "empty"),
        ("not-a-timestamp".to_string(), "invalid"),
        ((crate::unix_timestamp() + 60).to_string(), "future"),
    ];

    for (heartbeat, label) in cases {
        let root_name = format!("test-mascot-invalid-psd-viewer-tui-{label}");
        let (config_path, activity_path) = seed_favorite_ensemble_config(&root_name);
        fs::write(&activity_path, heartbeat).expect("should write psd-viewer-tui heartbeat");

        let loaded = load_mascot_config(&config_path)
            .unwrap_or_else(|error| panic!("{label} heartbeat should be ignored: {error:#}"));

        assert!(loaded.favorite_ensemble_enabled);
    }
}
