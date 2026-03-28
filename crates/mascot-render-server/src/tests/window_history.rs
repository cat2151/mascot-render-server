use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::Pos2;
use mascot_render_core::{
    workspace_cache_root, BounceAnimationConfig, HeadHitbox, IdleSinkAnimationConfig, MascotConfig,
    SquashBounceAnimationConfig,
};

use mascot_render_server::window_history::{
    load_saved_window_position_for_paths, load_window_position, outer_position_for_anchor,
    save_window_position_for_paths, window_history_path, SavedWindowPosition, WindowHistoryTracker,
    WINDOW_HISTORY_SAVE_DEBOUNCE,
};

#[test]
fn window_history_round_trips_saved_position() {
    let path = workspace_cache_root().join("test-window-history/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history"));

    let mut tracker = WindowHistoryTracker::new(path.clone(), None);
    let now = Instant::now();
    tracker
        .observe(Pos2::new(120.0, 48.0), now)
        .expect("should observe position");
    tracker.flush().expect("should save position");

    let loaded = load_window_position(&path).expect("should read saved history");
    assert_eq!(loaded, Some(Pos2::new(120.0, 48.0)));
}

#[test]
fn invalid_window_history_is_reported() {
    let path = workspace_cache_root().join("test-window-history-invalid/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history-invalid"));
    fs::create_dir_all(workspace_cache_root().join("test-window-history-invalid"))
        .expect("should create temp directory");
    fs::write(&path, "{ invalid json").expect("should seed invalid history");

    let error = load_window_position(&path).expect_err("invalid history should fail");
    assert!(error.to_string().contains("failed to parse window history"));
}

#[test]
fn legacy_v1_window_history_reports_parse_error() {
    let path = workspace_cache_root().join("test-window-history-v1/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history-v1"));
    fs::create_dir_all(workspace_cache_root().join("test-window-history-v1"))
        .expect("should create temp directory");
    fs::write(
        &path,
        r#"{
  "version": 1,
  "outer_position": [120.0, 48.0],
  "updated_at": 123
}"#,
    )
    .expect("should seed legacy history");

    let error = load_window_position(&path).expect_err("legacy history should fail to parse");
    assert!(error.to_string().contains("failed to parse window history"));
}

#[test]
fn tracker_saves_after_position_stabilizes() {
    let path = workspace_cache_root().join("test-window-history-debounce/history_server.json");
    let _ = fs::remove_dir_all(workspace_cache_root().join("test-window-history-debounce"));

    let mut tracker = WindowHistoryTracker::new(path.clone(), None);
    let now = Instant::now();
    tracker
        .observe(Pos2::new(20.0, 30.0), now)
        .expect("should observe initial position");
    assert!(
        !path.exists(),
        "history should not be written before the debounce elapses"
    );

    tracker
        .observe(Pos2::new(20.0, 30.0), now + WINDOW_HISTORY_SAVE_DEBOUNCE)
        .expect("should observe stabilized position");

    let loaded = load_window_position(&path).expect("should read saved history");
    assert_eq!(loaded, Some(Pos2::new(20.0, 30.0)));
}

#[test]
fn outer_position_for_anchor_subtracts_anchor_and_frame_offsets() {
    assert_eq!(
        outer_position_for_anchor(
            Pos2::new(320.0, 240.0),
            eframe::egui::Vec2::new(36.0, 66.0),
            eframe::egui::Vec2::new(8.0, 30.0),
        ),
        Pos2::new(276.0, 144.0)
    );
}

#[test]
fn window_history_path_is_scoped_per_psd() {
    let left = window_history_path(&mascot_config("/workspace/a.zip", "body/front.psd"));
    let right = window_history_path(&mascot_config("/workspace/b.zip", "body/front.psd"));
    let different_psd = window_history_path(&mascot_config("/workspace/a.zip", "body/back.psd"));

    assert_ne!(left, right);
    assert_ne!(left, different_psd);
}

#[test]
fn window_history_path_caps_long_psd_names() {
    let long_name = format!("{}.psd", "a".repeat(300));
    let path = window_history_path(&mascot_config("/workspace/a.zip", &long_name));
    let file_name = path
        .file_name()
        .expect("history path should have a file name");
    let file_name = file_name.to_string_lossy();

    assert!(
        file_name.len() < 255,
        "history file name should stay within typical filesystem limits: {file_name}"
    );
}

#[test]
fn public_helpers_round_trip_saved_window_position() {
    let config = mascot_config("/workspace/a.zip", "body/front.psd");
    let path = window_history_path(&config);
    let _ = fs::remove_file(&path);

    save_window_position_for_paths(
        &config.zip_path,
        &config.psd_path_in_zip,
        SavedWindowPosition { x: 256.0, y: 144.0 },
    )
    .expect("should save window position via public helper");

    let loaded = load_saved_window_position_for_paths(&config.zip_path, &config.psd_path_in_zip)
        .expect("should load window position via public helper");
    assert_eq!(loaded, Some(SavedWindowPosition { x: 256.0, y: 144.0 }));
}

#[test]
fn favorite_gallery_uses_dedicated_window_history_path() {
    let mut config = mascot_config("/workspace/a.zip", "body/front.psd");
    let per_psd_path = window_history_path(&config);
    config.favorite_gallery_enabled = true;

    let gallery_path = window_history_path(&config);
    assert_ne!(gallery_path, per_psd_path);
    assert_eq!(
        gallery_path.file_name().and_then(|value| value.to_str()),
        Some("history_server_favorite_gallery.json")
    );
}

fn mascot_config(zip_path: &str, psd_path_in_zip: &str) -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("/workspace/render.png"),
        scale: Some(1.0),
        favorite_gallery_scale: None,
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        display_diff_path: None,
        always_bouncing: false,
        always_bend: false,
        favorite_gallery_enabled: false,
        transparent_background_click_through: false,
        flash_blue_background_on_transparent_input: true,
        head_hitbox: HeadHitbox::default(),
        bounce: BounceAnimationConfig::default(),
        squash_bounce: SquashBounceAnimationConfig::default(),
        always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
    }
}
