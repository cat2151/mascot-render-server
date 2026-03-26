use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{apply_favorite_variation, apply_favorite_window_position, App};
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
fn favorite_entry_equality_includes_saved_state() {
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

    assert_ne!(left, right);
}

#[test]
fn favorite_saved_state_treats_negative_zero_like_zero() {
    let left = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
        mascot_scale: Some(-0.0),
        window_position: Some([-0.0, 20.0]),
    };
    let right = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
        mascot_scale: Some(0.0),
        window_position: Some([0.0, 20.0]),
    };

    assert!(left.same_saved_state_as(&right));
}

#[test]
fn favorites_deduplicate_only_exact_same_saved_state() {
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
psd_file_name = "body-copy.psd"
visibility_overrides = [{ layer_index = 3, visible = false }]
mascot_scale = 0.75
window_position = [120.0, 48.0]
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
fn favorites_allow_default_and_custom_layer_states_for_same_psd() {
    let root = workspace_cache_root().join("test-favorites-default-and-custom");
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

#[test]
fn selected_preview_png_path_prefers_favorite_preview_while_favorites_are_visible() {
    let mut app = App::loading(None);
    let current_preview = PathBuf::from("/cache/current.png");
    let favorite_preview = PathBuf::from("/cache/favorite.png");
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: None,
        window_position: None,
    };
    app.zip_entries = vec![ZipEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psds: vec![PsdEntry {
            path: PathBuf::from("/cache/a/body.psd"),
            file_name: "body.psd".to_string(),
            rendered_png_path: Some(favorite_preview.clone()),
            ..PsdEntry::default()
        }],
        ..ZipEntry::default()
    }];
    app.set_current_preview_png_path_for_test(Some(current_preview.clone()));
    app.set_favorites_for_test(
        vec![favorite.clone()],
        HashMap::from([(favorite.key(), (0, 0))]),
    );

    assert_eq!(
        app.selected_preview_png_path(),
        Some(current_preview.as_path())
    );

    app.toggle_favorites_view();

    assert_eq!(
        app.selected_preview_png_path(),
        Some(favorite_preview.as_path())
    );
}

#[test]
fn sync_selected_favorite_preview_uses_default_png_for_visible_favorite() {
    let preview_path = PathBuf::from("/cache/favorite.png");
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: None,
        window_position: None,
    };
    let mut app = App::loading(None);
    app.zip_entries = vec![ZipEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psds: vec![PsdEntry {
            path: PathBuf::from("/cache/a/body.psd"),
            file_name: "body.psd".to_string(),
            rendered_png_path: Some(preview_path.clone()),
            ..PsdEntry::default()
        }],
        ..ZipEntry::default()
    }];
    app.set_favorites_for_test(
        vec![favorite.clone()],
        HashMap::from([(favorite.key(), (0, 0))]),
    );

    app.sync_selected_favorite_preview_for_test()
        .expect("should sync selected favorite preview");

    assert_eq!(
        app.favorites_preview_png_path_for_test(),
        Some(preview_path.as_path())
    );
}

#[test]
fn favorites_view_prefers_exact_current_state_when_same_psd_has_multiple_entries() {
    let custom_favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 4,
            visible: true,
        }],
        mascot_scale: None,
        window_position: None,
    };
    let default_favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: None,
        window_position: None,
    };
    let mut app = App::loading(None);
    app.zip_entries = vec![ZipEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        extracted_dir: PathBuf::from("/cache/a"),
        psds: vec![sample_psd("/cache/a/a/body.psd", "body.psd")],
        ..ZipEntry::default()
    }];
    app.selected_zip_index = 0;
    app.selected_psd_index = 0;
    app.refresh_selected_psd_state_for_test()
        .expect("should build current psd document");
    app.set_favorites_for_test(
        vec![custom_favorite, default_favorite],
        HashMap::from([(
            FavoriteEntry {
                zip_path: PathBuf::from("/workspace/a.zip"),
                psd_path_in_zip: PathBuf::from("a/body.psd"),
                psd_file_name: "body.psd".to_string(),
                visibility_overrides: Vec::new(),
                mascot_scale: None,
                window_position: None,
            }
            .key(),
            (0, 0),
        )]),
    );

    app.toggle_favorites_view();

    assert_eq!(app.selected_favorite_selection(), Some(1));
}

#[test]
fn refresh_rebuilds_favorite_preview_when_visible() {
    let root = workspace_cache_root().join("test-favorite-preview-refresh");
    let preview_path = root.join("favorite-refresh.png");
    let favorite = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale: None,
        window_position: None,
    };
    let mut app = App::loading(None);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("should create temp preview directory");
    write_test_png(&preview_path);
    app.zip_entries = vec![ZipEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psds: vec![PsdEntry {
            path: PathBuf::from("/cache/a/body.psd"),
            file_name: "body.psd".to_string(),
            rendered_png_path: Some(preview_path.clone()),
            ..PsdEntry::default()
        }],
        ..ZipEntry::default()
    }];
    app.set_favorites_for_test(
        vec![favorite.clone()],
        HashMap::from([(favorite.key(), (0, 0))]),
    );

    app.toggle_favorites_view();
    app.refresh_selected_psd_state_for_test()
        .expect("refresh should rebuild favorites preview");

    assert_eq!(
        app.favorites_preview_png_path_for_test(),
        Some(preview_path.as_path())
    );
    assert_eq!(
        app.selected_preview_png_path(),
        Some(preview_path.as_path())
    );
    let _ = fs::remove_dir_all(&root);
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

fn write_test_png(path: &std::path::Path) {
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
