use std::fs;
use std::path::{Path, PathBuf};

use crate::favorites::{favorites_path, load_favorites, save_favorites, FavoriteEntry};
use mascot_render_core::{workspace_cache_root, LayerVisibilityOverride, PsdEntry};

#[path = "favorites_app.rs"]
mod favorites_app;
#[path = "favorites_entries.rs"]
mod favorites_entries;

#[test]
fn favorites_round_trip_as_toml() {
    let root = workspace_cache_root().join("test-favorites-round-trip");
    let path = root.join("favorites/favorites.toml");
    let _ = fs::remove_dir_all(&root);

    let favorites = vec![
        FavoriteEntry {
            zip_path: PathBuf::from("/workspace/a.zip"),
            psd_path_in_zip: PathBuf::from("a/body.psd"),
            psd_file_name: "body.psd".to_string(),
            visibility_overrides: vec![LayerVisibilityOverride {
                layer_index: 7,
                visible: false,
            }],
            mascot_scale: Some(0.75),
            window_position: Some([120.0, 48.0]),
            favorite_ensemble_position: Some([300.0, 90.0]),
        },
        FavoriteEntry {
            zip_path: PathBuf::from("/workspace/b.zip"),
            psd_path_in_zip: PathBuf::from("b/face.psd"),
            psd_file_name: "face.psd".to_string(),
            visibility_overrides: vec![LayerVisibilityOverride {
                layer_index: 3,
                visible: true,
            }],
            mascot_scale: Some(0.5),
            window_position: Some([300.0, 90.0]),
            favorite_ensemble_position: Some([180.0, 24.0]),
        },
    ];

    save_favorites(&path, &favorites).expect("should write favorites");
    let saved = fs::read_to_string(&path).expect("should read favorites toml");
    assert!(!saved.contains("version ="));

    let loaded = load_favorites(&path).expect("should read favorites");
    assert_eq!(loaded, favorites);
}

#[test]
fn favorites_path_uses_dedicated_file_name() {
    assert_eq!(
        favorites_path()
            .file_name()
            .and_then(|value| value.to_str()),
        Some("favorites.toml")
    );
}

#[test]
fn favorites_deduplicate_by_visibility_and_keep_latest_scale_and_position() {
    let root = workspace_cache_root().join("test-favorites-deduplicate");
    let path = root.join("favorites/favorites.toml");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites file should have a parent"))
        .expect("should create temp directory");

    fs::write(
        &path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
visibility_overrides = [{ layer_index = 3, visible = false }]
mascot_scale = 0.75
window_position = [120.0, 48.0]
favorite_ensemble_position = [300.0, 90.0]

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body-copy.psd"
visibility_overrides = [{ layer_index = 3, visible = false }]
mascot_scale = 1.25
window_position = [300.0, 90.0]
favorite_ensemble_position = [180.0, 24.0]
"#,
    )
    .expect("should seed duplicate favorites");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].psd_file_name, "body-copy.psd");
    assert_eq!(
        loaded[0].visibility_overrides,
        vec![LayerVisibilityOverride {
            layer_index: 3,
            visible: false,
        }]
    );
    assert_eq!(loaded[0].mascot_scale, Some(1.25));
    assert_eq!(loaded[0].window_position, Some([300.0, 90.0]));
    assert_eq!(loaded[0].favorite_ensemble_position, Some([180.0, 24.0]));
}

#[test]
fn favorites_allow_default_and_custom_layer_states_for_same_psd() {
    let root = workspace_cache_root().join("test-favorites-default-and-custom");
    let path = root.join("favorites/favorites.toml");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites file should have a parent"))
        .expect("should create temp directory");

    fs::write(
        &path,
        r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
visibility_overrides = [{ layer_index = 4, visible = true }]
"#,
    )
    .expect("should seed favorites with default and custom states");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 2);
    assert!(loaded[0].visibility_overrides.is_empty());
    assert_eq!(
        loaded[1].visibility_overrides,
        vec![LayerVisibilityOverride {
            layer_index: 4,
            visible: true,
        }]
    );
}

#[test]
fn favorites_with_legacy_version_field_are_rejected() {
    let root = workspace_cache_root().join("test-favorites-legacy-version");
    let path = root.join("favorites/favorites.toml");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(path.parent().expect("favorites file should have a parent"))
        .expect("should create temp directory");

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
    .expect("should seed legacy-format favorite");

    let loaded = load_favorites(&path).expect("should ignore legacy favorites");
    assert!(loaded.is_empty());
}

fn sample_psd(path: &str, file_name: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        file_name: file_name.to_string(),
        ..PsdEntry::default()
    }
}

fn remove_file_if_exists(path: &Path) {
    if path.exists() {
        fs::remove_file(path).expect("test cleanup should remove window history file");
    }
}

fn write_test_png(path: &Path) {
    fs::write(
        path,
        [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 255,
            255, 255, 127, 0, 9, 251, 3, 253, 42, 134, 227, 138, 0, 0, 0, 0, 73, 69, 78, 68, 174,
            66, 96, 130,
        ],
    )
    .expect("should write test png");
}
