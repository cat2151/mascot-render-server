use super::*;

#[test]
fn load_mascot_config_disables_favorite_ensemble_while_psd_viewer_tui_is_active() {
    let (config_path, activity_path) =
        seed_favorite_ensemble_config("test-mascot-active-psd-viewer-tui");
    fs::write(&activity_path, crate::unix_timestamp().to_string())
        .expect("should write psd-viewer-tui heartbeat");

    let loaded = load_mascot_config(&config_path).expect("config should load");

    assert!(
        !loaded.favorite_ensemble_enabled,
        "psd-viewer-tui activity should temporarily disable favorite ensemble"
    );
}

#[test]
fn load_mascot_config_reenables_favorite_ensemble_after_psd_viewer_tui_activity_ends() {
    let (config_path, activity_path) =
        seed_favorite_ensemble_config("test-mascot-ended-psd-viewer-tui");
    fs::write(&activity_path, crate::unix_timestamp().to_string())
        .expect("should write psd-viewer-tui heartbeat");

    let active = load_mascot_config(&config_path).expect("config should load while active");
    assert!(!active.favorite_ensemble_enabled);

    fs::remove_file(&activity_path).expect("should remove psd-viewer-tui heartbeat");
    let inactive =
        load_mascot_config(&config_path).expect("config should reload after heartbeat removal");
    assert!(inactive.favorite_ensemble_enabled);

    fs::write(
        &activity_path,
        crate::unix_timestamp().saturating_sub(10).to_string(),
    )
    .expect("should write stale psd-viewer-tui heartbeat");
    let stale = load_mascot_config(&config_path).expect("config should load with stale heartbeat");
    assert!(stale.favorite_ensemble_enabled);
}

#[test]
fn load_mascot_config_ignores_invalid_psd_viewer_tui_heartbeats() {
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

        assert!(
            loaded.favorite_ensemble_enabled,
            "{label} heartbeat should not disable favorite ensemble"
        );
    }
}
