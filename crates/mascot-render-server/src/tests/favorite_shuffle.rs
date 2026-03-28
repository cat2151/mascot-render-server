use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::favorite_shuffle::{
    build_playlist, favorites_path_for, load_favorites, suppress_rotation_for_active_display_diff,
    suppress_rotation_for_psd_viewer_tui_activity_path, FavoriteEntry, FavoriteShufflePlaylist,
    FAVORITE_SHUFFLE_INTERVAL,
};
use mascot_render_core::{
    psd_viewer_tui_activity_path, workspace_cache_root, BounceAnimationConfig, Core, CoreConfig,
    HeadHitbox, IdleSinkAnimationConfig, MascotConfig, SquashBounceAnimationConfig,
};

#[test]
fn favorite_shuffle_deduplicates_and_fills_missing_file_name() {
    let root = workspace_cache_root().join("test-favorite-shuffle-load");
    let path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites path should have a parent"))
        .expect("should create favorites directory");

    fs::write(
        &path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = ""

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body-renamed.psd"
"#,
    )
    .expect("should seed favorites cache");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].psd_file_name, "body.psd");
    assert_eq!(loaded[0].mascot_scale, None);
}

#[test]
fn favorite_shuffle_invalid_entry_is_rejected_during_parse() {
    let root = workspace_cache_root().join("test-favorite-shuffle-invalid-entry");
    let path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites path should have a parent"))
        .expect("should create favorites directory");

    fs::write(
        &path,
        r#"
[[favorites]]
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
"#,
    )
    .expect("should seed invalid favorites cache");

    let loaded =
        load_favorites(&path).expect("should load favorites while ignoring invalid entries");
    assert!(loaded.is_empty());
}

#[test]
fn favorite_shuffle_uses_dedicated_favorites_file_name() {
    let root = workspace_cache_root().join("test-favorite-shuffle-path");
    assert_eq!(
        favorites_path_for(&root),
        root.join("favorites/favorites.toml")
    );
}

#[test]
fn favorite_shuffle_rejects_legacy_version_field() {
    let root = workspace_cache_root().join("test-favorite-shuffle-legacy-version");
    let path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites path should have a parent"))
        .expect("should create favorites directory");

    fs::write(
        &path,
        r#"
version = 1

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
"#,
    )
    .expect("should seed legacy favorites cache");

    let loaded = load_favorites(&path).expect("should ignore legacy favorites cache");
    assert!(loaded.is_empty());
}

#[test]
fn favorite_shuffle_playlist_starts_with_a_different_favorite() {
    let favorites = vec![
        favorite("/workspace/a.zip", "a/body.psd", "body.psd"),
        favorite("/workspace/b.zip", "b/face.psd", "face.psd"),
        favorite("/workspace/c.zip", "c/pose.psd", "pose.psd"),
    ];
    let current_key = favorites[0].key();

    let playlist = build_playlist(&favorites, Some(&current_key), 1);
    let ordered: Vec<_> = playlist.into_iter().collect();

    assert_eq!(ordered.len(), favorites.len());
    assert_ne!(ordered[0].key(), current_key);

    let unique_keys: HashSet<_> = ordered.into_iter().map(|favorite| favorite.key()).collect();
    assert_eq!(unique_keys.len(), favorites.len());
}

#[test]
fn favorite_entry_equality_detects_saved_scale_changes() {
    let mut left = favorite("/workspace/a.zip", "a/body.psd", "body.psd");
    let mut right = favorite("/workspace/a.zip", "a/body.psd", "body.psd");
    right.mascot_scale = Some(1.25);

    assert_ne!(left, right);

    left.mascot_scale = Some(1.25);
    assert_eq!(left, right);
}

#[test]
fn favorite_shuffle_is_suppressed_while_previewing_an_edited_variation() {
    let mut config = mascot_config("/workspace/a.zip", "a/body.psd");
    config.display_diff_path = Some(PathBuf::from("/workspace/edited-variation.json"));

    assert!(suppress_rotation_for_active_display_diff(&config));
}

#[test]
fn favorite_shuffle_skips_loading_favorites_while_previewing_an_edited_variation() {
    let root = workspace_cache_root().join("test-favorite-shuffle-active-edit");
    let favorites_path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    create_invalid_favorites_path(&favorites_path);

    let now = Instant::now();
    let mut playlist = FavoriteShufflePlaylist::new_with_path(favorites_path, now);
    let mut config = mascot_config("/workspace/a.zip", "a/body.psd");
    config.display_diff_path = Some(PathBuf::from("/workspace/edited-variation.json"));

    let rotated = playlist
        .update(
            &Core::new(CoreConfig::default()),
            &root.join("mascot.toml"),
            &config,
            now + FAVORITE_SHUFFLE_INTERVAL,
        )
        .expect("active edit should pause favorite shuffle before reading favorites");

    assert!(!rotated);
}

#[test]
fn favorite_shuffle_still_reads_favorites_without_an_active_edit() {
    let root = workspace_cache_root().join("test-favorite-shuffle-normal-read");
    let favorites_path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    create_invalid_favorites_path(&favorites_path);

    let now = Instant::now();
    let mut playlist = FavoriteShufflePlaylist::new_with_path(favorites_path, now);
    let config = mascot_config("/workspace/a.zip", "a/body.psd");

    let error = playlist
        .update(
            &Core::new(CoreConfig::default()),
            &root.join("mascot.toml"),
            &config,
            now + FAVORITE_SHUFFLE_INTERVAL,
        )
        .expect_err("normal favorite shuffle should still try to read the favorites cache");

    assert!(
        error.to_string().contains("failed to read favorites"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn favorite_shuffle_is_suppressed_while_psd_viewer_tui_is_active() {
    let config_path = workspace_cache_root().join("test-favorite-shuffle-active-tui/mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    fs::remove_file(&activity_path).ok();
    fs::create_dir_all(
        activity_path
            .parent()
            .expect("activity path should have a parent directory"),
    )
    .expect("should create activity directory");
    fs::write(&activity_path, "100").expect("should write activity heartbeat");

    assert!(
        suppress_rotation_for_psd_viewer_tui_activity_path(&activity_path, 104)
            .expect("recent heartbeat should pause favorite shuffle")
    );
}

#[test]
fn favorite_shuffle_ignores_stale_psd_viewer_tui_activity() {
    let config_path = workspace_cache_root().join("test-favorite-shuffle-stale-tui/mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    fs::remove_file(&activity_path).ok();
    fs::create_dir_all(
        activity_path
            .parent()
            .expect("activity path should have a parent directory"),
    )
    .expect("should create activity directory");
    fs::write(&activity_path, "100").expect("should write activity heartbeat");

    assert!(
        !suppress_rotation_for_psd_viewer_tui_activity_path(&activity_path, 106)
            .expect("stale heartbeat should not pause favorite shuffle")
    );
}

#[test]
fn favorite_shuffle_ignores_missing_psd_viewer_tui_activity() {
    let config_path = workspace_cache_root().join("test-favorite-shuffle-missing-tui/mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    fs::remove_file(&activity_path).ok();

    assert!(
        !suppress_rotation_for_psd_viewer_tui_activity_path(&activity_path, 106)
            .expect("missing heartbeat should not pause favorite shuffle")
    );
}

#[test]
fn favorite_shuffle_ignores_future_psd_viewer_tui_activity() {
    let config_path = workspace_cache_root().join("test-favorite-shuffle-future-tui/mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    fs::remove_file(&activity_path).ok();
    fs::create_dir_all(
        activity_path
            .parent()
            .expect("activity path should have a parent directory"),
    )
    .expect("should create activity directory");
    fs::write(&activity_path, "107").expect("should write activity heartbeat");

    assert!(
        !suppress_rotation_for_psd_viewer_tui_activity_path(&activity_path, 106)
            .expect("future heartbeat should not pause favorite shuffle")
    );
}

#[test]
fn favorite_shuffle_ignores_unreadable_psd_viewer_tui_activity() {
    let config_path =
        workspace_cache_root().join("test-favorite-shuffle-unreadable-tui/mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    let _cleanup = TestFixtureCleanup(activity_path.clone());
    fs::remove_dir_all(
        activity_path
            .parent()
            .expect("activity path should have a parent directory"),
    )
    .ok();
    fs::create_dir_all(&activity_path)
        .expect("should create directory at activity path to simulate unreadable heartbeat file");

    assert!(
        !suppress_rotation_for_psd_viewer_tui_activity_path(&activity_path, 106)
            .expect("unreadable heartbeat should not pause favorite shuffle")
    );
}

#[test]
fn favorite_shuffle_skips_loading_favorites_while_psd_viewer_tui_is_active() {
    let root = workspace_cache_root().join("test-favorite-shuffle-active-tui-read");
    let favorites_path = favorites_path_for(&root);
    let config_path = root.join("mascot.toml");
    let activity_path = psd_viewer_tui_activity_path(&config_path);
    let now_unix_timestamp = 100;
    let _ = fs::remove_dir_all(&root);
    create_invalid_favorites_path(&favorites_path);
    fs::create_dir_all(
        activity_path
            .parent()
            .expect("activity path should have a parent directory"),
    )
    .expect("should create activity directory");
    fs::write(&activity_path, now_unix_timestamp.to_string())
        .expect("should write activity heartbeat");

    let now = Instant::now();
    let mut playlist = FavoriteShufflePlaylist::new_with_path(favorites_path, now);
    let config = mascot_config("/workspace/a.zip", "a/body.psd");

    let rotated = playlist
        .update_with_unix_timestamp_for_test(
            &Core::new(CoreConfig::default()),
            &config_path,
            &config,
            now + FAVORITE_SHUFFLE_INTERVAL,
            now_unix_timestamp,
        )
        .expect("active psd-viewer-tui should pause favorite shuffle before reading favorites");

    assert!(!rotated);
}

#[test]
fn favorite_shuffle_loads_saved_mascot_scale_per_favorite() {
    let root = workspace_cache_root().join("test-favorite-shuffle-load-scale");
    let path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites path should have a parent"))
        .expect("should create favorites directory");

    fs::write(
        &path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
mascot_scale = 1.75
"#,
    )
    .expect("should seed favorites cache");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].mascot_scale, Some(1.75));
}

#[test]
fn favorite_shuffle_discards_invalid_saved_mascot_scales() {
    let root = workspace_cache_root().join("test-favorite-shuffle-invalid-scale");
    let path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites path should have a parent"))
        .expect("should create favorites directory");

    fs::write(
        &path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
mascot_scale = -1.0

[[favorites]]
zip_path = "/workspace/b.zip"
psd_path_in_zip = "b/face.psd"
psd_file_name = "face.psd"
mascot_scale = 0.0

[[favorites]]
zip_path = "/workspace/c.zip"
psd_path_in_zip = "c/pose.psd"
psd_file_name = "pose.psd"
mascot_scale = inf

[[favorites]]
zip_path = "/workspace/d.zip"
psd_path_in_zip = "d/wink.psd"
psd_file_name = "wink.psd"
mascot_scale = nan
"#,
    )
    .expect("should seed favorites cache");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 4);
    assert!(loaded
        .iter()
        .all(|favorite| favorite.mascot_scale.is_none()));
}

#[test]
fn favorite_shuffle_persists_scale_for_matching_favorite_without_losing_other_fields() {
    let root = workspace_cache_root().join("test-favorite-shuffle-persist-scale");
    let favorites_path = favorites_path_for(&root);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(
        favorites_path
            .parent()
            .expect("favorites path should have a parent directory"),
    )
    .expect("should create favorites directory");

    fs::write(
        &favorites_path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
visibility_overrides = [{ layer_index = 3, visible = false }]
window_position = [120.0, 48.0]
favorite_ensemble_position = [360.0, 12.0]

[[favorites]]
zip_path = "/workspace/b.zip"
psd_path_in_zip = "b/face.psd"
psd_file_name = "face.psd"
mascot_scale = 0.8
"#,
    )
    .expect("should seed favorites cache");

    let playlist = FavoriteShufflePlaylist::new_with_path(favorites_path.clone(), Instant::now());
    let updated = playlist
        .persist_scale_for_current_config(&mascot_config("/workspace/a.zip", "a/body.psd"), 1.25)
        .expect("should persist the saved mascot scale");
    assert!(updated, "matching favorite should be updated");

    let reloaded = load_favorites(&favorites_path).expect("should reload favorites");
    assert_eq!(reloaded.len(), 2);
    assert_eq!(reloaded[0].mascot_scale, Some(1.25));
    assert_eq!(reloaded[1].mascot_scale, Some(0.8));

    let raw = fs::read_to_string(&favorites_path).expect("should read rewritten favorites");
    let parsed: toml::Value = toml::from_str(&raw).expect("rewritten favorites should stay valid");
    let favorites = parsed["favorites"]
        .as_array()
        .expect("favorites should remain an array");
    assert_eq!(
        favorites[0]["window_position"].as_array(),
        Some(&vec![toml::Value::from(120.0), toml::Value::from(48.0)]),
        "window position should be preserved: {raw}"
    );
    assert_eq!(
        favorites[0]["favorite_ensemble_position"].as_array(),
        Some(&vec![toml::Value::from(360.0), toml::Value::from(12.0)]),
        "favorite ensemble position should be preserved: {raw}"
    );
    assert_eq!(
        favorites[0]["visibility_overrides"][0]["layer_index"].as_integer(),
        Some(3),
        "visibility override should be preserved: {raw}"
    );
    assert_eq!(
        favorites[0]["visibility_overrides"][0]["visible"].as_bool(),
        Some(false),
        "visibility override should be preserved: {raw}"
    );
}

fn favorite(zip_path: &str, psd_path_in_zip: &str, psd_file_name: &str) -> FavoriteEntry {
    FavoriteEntry {
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        psd_file_name: psd_file_name.to_string(),
        mascot_scale: None,
    }
}

fn mascot_config(zip_path: &str, psd_path_in_zip: &str) -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("/workspace/render.png"),
        scale: Some(1.0),
        favorite_ensemble_scale: None,
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        display_diff_path: None,
        always_bouncing: false,
        always_bend: false,
        favorite_ensemble_enabled: false,
        transparent_background_click_through: false,
        flash_blue_background_on_transparent_input: true,
        head_hitbox: HeadHitbox::default(),
        bounce: BounceAnimationConfig::default(),
        squash_bounce: SquashBounceAnimationConfig::default(),
        always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
    }
}

fn create_invalid_favorites_path(favorites_path: &std::path::Path) {
    let favorites_dir = favorites_path
        .parent()
        .expect("favorites path should have a parent directory");
    fs::create_dir_all(favorites_dir).expect("should create favorites directory");
    fs::create_dir(favorites_path).expect(
        "should create directory at favorites file path to simulate invalid favorites file",
    );
}

/// RAII guard that removes a temporary test fixture path on drop.
struct TestFixtureCleanup(PathBuf);

impl Drop for TestFixtureCleanup {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}
