use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::app::apply_favorite_variation;
use crate::favorites::{favorite_selection_lookup, load_favorites, save_favorites, FavoriteEntry};
use mascot_render_core::workspace_cache_root;
use mascot_render_core::{DisplayDiff, LayerVisibilityOverride, PsdEntry, ZipEntry};

#[test]
fn favorites_round_trip_as_toml() {
    let root = workspace_cache_root().join("test-favorites-round-trip");
    let path = root.join("favorites/psd-viewer-tui.toml");
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
        },
        FavoriteEntry {
            zip_path: PathBuf::from("/workspace/b.zip"),
            psd_path_in_zip: PathBuf::from("b/face.psd"),
            psd_file_name: "face.psd".to_string(),
            visibility_overrides: vec![LayerVisibilityOverride {
                layer_index: 3,
                visible: true,
            }],
        },
    ];

    save_favorites(&path, &favorites).expect("should write favorites");

    let loaded = load_favorites(&path).expect("should read favorites");
    assert_eq!(loaded, favorites);
}

#[test]
fn favorite_entry_equality_ignores_display_name() {
    let left = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
    };
    let right = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body-renamed.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 1,
            visible: true,
        }],
    };

    assert_eq!(left, right);
}

#[test]
fn favorites_deduplicate_entries_by_zip_and_psd_path() {
    let root = workspace_cache_root().join("test-favorites-deduplicate");
    let path = root.join("favorites/psd-viewer-tui.toml");
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
visibility_overrides = [{ layer_index = 3, visible = false }]

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body-renamed.psd"
visibility_overrides = [{ layer_index = 4, visible = true }]
"#,
    )
    .expect("should seed duplicate favorites");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].psd_file_name, "body.psd");
    assert_eq!(
        loaded[0].visibility_overrides,
        vec![LayerVisibilityOverride {
            layer_index: 3,
            visible: false,
        }]
    );
}

#[test]
fn favorites_without_visibility_overrides_still_load() {
    let root = workspace_cache_root().join("test-favorites-legacy-load");
    let path = root.join("favorites/psd-viewer-tui.toml");
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
    .expect("should seed legacy favorite");

    let loaded = load_favorites(&path).expect("should load legacy favorites");
    assert_eq!(loaded.len(), 1);
    assert!(loaded[0].visibility_overrides.is_empty());
}

#[test]
fn favorite_selection_matches_zip_path_and_psd_path_in_zip() {
    let zip_entries = vec![
        ZipEntry {
            zip_path: PathBuf::from("/workspace/a.zip"),
            extracted_dir: PathBuf::from("/cache/a"),
            psds: vec![sample_psd("/cache/a/a/body.psd", "body.psd")],
            ..ZipEntry::default()
        },
        ZipEntry {
            zip_path: PathBuf::from("/workspace/b.zip"),
            extracted_dir: PathBuf::from("/cache/b"),
            psds: vec![sample_psd("/cache/b/b/face.psd", "face.psd")],
            ..ZipEntry::default()
        },
    ];

    let selection = favorite_selection_lookup(&zip_entries)
        .get(
            &FavoriteEntry {
                zip_path: PathBuf::from("/workspace/b.zip"),
                psd_path_in_zip: PathBuf::from("b/face.psd"),
                psd_file_name: "face.psd".to_string(),
                visibility_overrides: Vec::new(),
            }
            .key(),
        )
        .copied();

    assert_eq!(selection, Some((1, 0)));
}

#[test]
fn apply_favorite_variation_restores_saved_visibility() {
    let mut variations = HashMap::new();
    let psd_path = PathBuf::from("/cache/a/body.psd");
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 9,
            visible: false,
        }],
    };

    apply_favorite_variation(&mut variations, &psd_path, &favorite);

    assert_eq!(
        variations.get(&psd_path),
        Some(&DisplayDiff {
            version: 1,
            visibility_overrides: favorite.visibility_overrides.clone(),
        })
    );
}

#[test]
fn apply_favorite_variation_clears_previous_override_when_favorite_is_default() {
    let psd_path = PathBuf::from("/cache/a/body.psd");
    let mut variations = HashMap::from([(
        psd_path.clone(),
        DisplayDiff {
            version: 1,
            visibility_overrides: vec![LayerVisibilityOverride {
                layer_index: 2,
                visible: false,
            }],
        },
    )]);
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
    };

    apply_favorite_variation(&mut variations, &psd_path, &favorite);

    assert!(!variations.contains_key(&psd_path));
}

fn sample_psd(path: &str, file_name: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        file_name: file_name.to_string(),
        ..PsdEntry::default()
    }
}
