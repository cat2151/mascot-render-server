use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::favorite_shuffle::{
    build_playlist, favorites_path_for, load_favorites, suppress_rotation_for_active_display_diff,
    FavoriteEntry, FavoriteShufflePlaylist, FAVORITE_SHUFFLE_INTERVAL,
};
use mascot_render_core::{
    workspace_cache_root, BounceAnimationConfig, Core, CoreConfig, HeadHitbox,
    IdleSinkAnimationConfig, MascotConfig, SquashBounceAnimationConfig,
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

fn favorite(zip_path: &str, psd_path_in_zip: &str, psd_file_name: &str) -> FavoriteEntry {
    FavoriteEntry {
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        psd_file_name: psd_file_name.to_string(),
    }
}

fn mascot_config(zip_path: &str, psd_path_in_zip: &str) -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("/workspace/render.png"),
        scale: Some(1.0),
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        display_diff_path: None,
        always_bouncing: false,
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
