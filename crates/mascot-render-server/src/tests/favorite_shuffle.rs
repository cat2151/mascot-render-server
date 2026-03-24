use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::favorite_shuffle::{build_playlist, favorites_path_for, load_favorites, FavoriteEntry};
use mascot_render_core::workspace_cache_root;

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
version = 1

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
version = 1

[[favorites]]
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
"#,
    )
    .expect("should seed invalid favorites cache");

    let loaded = load_favorites(&path).expect("invalid favorites cache should be ignored");
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

fn favorite(zip_path: &str, psd_path_in_zip: &str, psd_file_name: &str) -> FavoriteEntry {
    FavoriteEntry {
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        psd_file_name: psd_file_name.to_string(),
    }
}
