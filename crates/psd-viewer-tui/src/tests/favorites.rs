use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{apply_favorite_variation, apply_favorite_window_position};
use crate::favorites::{favorite_selection_lookup, load_favorites, save_favorites, FavoriteEntry};
use crate::is_favorites_toggle_key;
use mascot_render_core::workspace_cache_root;
use mascot_render_core::{DisplayDiff, LayerVisibilityOverride, PsdEntry, ZipEntry};
use mascot_render_server::{load_saved_window_position_for_paths, window_history_path_for_paths};

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
            mascot_scale: Some(0.75),
            window_position: Some([120.0, 48.0]),
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
        },
    ];

    save_favorites(&path, &favorites).expect("should write favorites");

    let loaded = load_favorites(&path).expect("should read favorites");
    assert_eq!(loaded, favorites);
}

#[test]
fn favorite_entry_equality_depends_only_on_zip_and_psd_path() {
    let left = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
        mascot_scale: Some(0.8),
        window_position: Some([10.0, 20.0]),
    };
    let right = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body-renamed.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 1,
            visible: true,
        }],
        mascot_scale: Some(1.2),
        window_position: Some([40.0, 50.0]),
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
mascot_scale = 0.75
window_position = [120.0, 48.0]

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body-renamed.psd"
visibility_overrides = [{ layer_index = 4, visible = true }]
mascot_scale = 0.5
window_position = [300.0, 90.0]
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
    assert_eq!(loaded[0].mascot_scale, Some(0.75));
    assert_eq!(loaded[0].window_position, Some([120.0, 48.0]));
}

#[test]
fn favorites_without_new_fields_still_load() {
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
    assert_eq!(loaded[0].mascot_scale, None);
    assert_eq!(loaded[0].window_position, None);
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
                mascot_scale: None,
                window_position: None,
            }
            .key(),
        )
        .copied();

    assert_eq!(selection, Some((1, 0)));
}

#[test]
fn favorites_toggle_accepts_v_always_and_esc_only_when_visible() {
    let plain_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
    let plain_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let ctrl_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL);

    assert!(is_favorites_toggle_key(&plain_v, false));
    assert!(is_favorites_toggle_key(&plain_v, true));
    assert!(!is_favorites_toggle_key(&plain_esc, false));
    assert!(is_favorites_toggle_key(&plain_esc, true));
    assert!(!is_favorites_toggle_key(&ctrl_v, true));
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
        mascot_scale: Some(0.6),
        window_position: Some([150.0, 75.0]),
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
        mascot_scale: None,
        window_position: None,
    };

    apply_favorite_variation(&mut variations, &psd_path, &favorite);

    assert!(!variations.contains_key(&psd_path));
}

#[test]
fn apply_favorite_window_position_persists_saved_coordinates() {
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/test-favorite-window-position-a.zip"),
        psd_path_in_zip: PathBuf::from("nested/body-a.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: Some(0.8),
        window_position: Some([222.0, 111.0]),
    };
    let history_path = window_history_path_for_paths(&favorite.zip_path, &favorite.psd_path_in_zip);
    remove_file_if_exists(&history_path);

    apply_favorite_window_position(&favorite).expect("should save favorite window position");

    let loaded =
        load_saved_window_position_for_paths(&favorite.zip_path, &favorite.psd_path_in_zip)
            .expect("should load saved favorite window position");
    assert_eq!(
        loaded.map(|position| [position.x, position.y]),
        favorite.window_position
    );
    remove_file_if_exists(&history_path);
}

#[test]
fn apply_favorite_window_position_ignores_missing_coordinates() {
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/test-favorite-window-position-b.zip"),
        psd_path_in_zip: PathBuf::from("nested/body-b.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: Some(0.8),
        window_position: None,
    };

    assert!(!apply_favorite_window_position(&favorite).expect("should ignore missing coordinates"));
}

fn sample_psd(path: &str, file_name: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        file_name: file_name.to_string(),
        ..PsdEntry::default()
    }
}

fn remove_file_if_exists(path: &std::path::Path) {
    if path.exists() {
        fs::remove_file(path).expect("test cleanup should remove window history file");
    }
}
