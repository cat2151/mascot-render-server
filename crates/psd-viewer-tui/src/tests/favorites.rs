use std::fs;
use std::path::PathBuf;

use crate::favorites::{favorite_selection_lookup, load_favorites, save_favorites, FavoriteEntry};
use mascot_render_core::workspace_cache_root;
use mascot_render_core::{PsdEntry, ZipEntry};

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
        },
        FavoriteEntry {
            zip_path: PathBuf::from("/workspace/b.zip"),
            psd_path_in_zip: PathBuf::from("b/face.psd"),
            psd_file_name: "face.psd".to_string(),
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
    };
    let right = FavoriteEntry {
        zip_path: PathBuf::from("/workspace/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        psd_file_name: "body-renamed.psd".to_string(),
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

[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body-renamed.psd"
"#,
    )
    .expect("should seed duplicate favorites");

    let loaded = load_favorites(&path).expect("should load favorites");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].psd_file_name, "body.psd");
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
            }
            .key(),
        )
        .copied();

    assert_eq!(selection, Some((1, 0)));
}

fn sample_psd(path: &str, file_name: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        file_name: file_name.to_string(),
        ..PsdEntry::default()
    }
}
