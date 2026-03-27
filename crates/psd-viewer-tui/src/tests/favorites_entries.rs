use std::path::PathBuf;

use crate::favorites::{favorite_selection_lookup, FavoriteEntry};
use mascot_render_core::{LayerVisibilityOverride, ZipEntry};

use super::sample_psd;

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
fn favorite_identity_ignores_scale_and_window_position() {
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
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: vec![LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
        mascot_scale: Some(1.2),
        window_position: Some([40.0, 50.0]),
    };

    assert!(left.same_favorite_identity_as(&right));
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
