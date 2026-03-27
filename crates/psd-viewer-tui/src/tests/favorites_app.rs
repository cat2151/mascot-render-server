use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{apply_favorite_variation, apply_favorite_window_position, App, FocusPane};
use crate::favorites::FavoriteEntry;
use crate::{is_favorite_save_key, is_favorites_toggle_key};
use mascot_render_core::{
    workspace_cache_root, DisplayDiff, LayerVisibilityOverride, PsdEntry, ZipEntry,
};
use mascot_render_server::{load_saved_window_position_for_paths, window_history_path_for_paths};

use super::{remove_file_if_exists, sample_psd, write_test_png};

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
fn favorite_save_key_accepts_f_from_library_and_layer_when_favorites_hidden() {
    let plain_f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
    let ctrl_f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);

    assert!(is_favorite_save_key(&plain_f, FocusPane::Library, false));
    assert!(is_favorite_save_key(&plain_f, FocusPane::Layer, false));
    assert!(!is_favorite_save_key(&plain_f, FocusPane::Library, true));
    assert!(!is_favorite_save_key(&ctrl_f, FocusPane::Layer, false));
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
fn upsert_favorite_updates_existing_entry_when_only_scale_and_position_change() {
    let existing = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 4,
            visible: true,
        }],
        mascot_scale: Some(0.75),
        window_position: Some([120.0, 48.0]),
    };
    let updated = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 4,
            visible: true,
        }],
        mascot_scale: Some(1.25),
        window_position: Some([300.0, 90.0]),
    };
    let mut app = App::loading(None);
    app.set_favorites_for_test(vec![existing], HashMap::new());

    assert!(app.upsert_favorite_for_test(updated.clone()));
    assert_eq!(app.favorite_entries_for_test(), &[updated]);
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
