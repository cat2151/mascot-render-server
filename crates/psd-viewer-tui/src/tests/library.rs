use std::collections::HashMap;
use std::path::PathBuf;

use mascot_render_core::{PsdEntry, ZipEntry};

use crate::app::library::{
    build_library_rows, first_psd_selection, selected_flat_index, selected_row_index,
    selection_from_flat_index, selection_from_psd_path, LibraryRow,
};
use crate::app::App;
use crate::favorites::FavoriteEntry;

#[test]
fn builds_tree_rows_with_zip_headers_and_psd_children() {
    let zip_entries = sample_zip_entries();

    let rows = build_library_rows(&zip_entries);

    assert_eq!(
        rows,
        vec![
            LibraryRow::ZipHeader { zip_index: 0 },
            LibraryRow::PsdItem {
                zip_index: 0,
                psd_index: 0,
            },
            LibraryRow::PsdItem {
                zip_index: 0,
                psd_index: 1,
            },
            LibraryRow::ZipHeader { zip_index: 1 },
            LibraryRow::PsdItem {
                zip_index: 1,
                psd_index: 0,
            },
        ]
    );
}

#[test]
fn maps_flat_selection_to_tree_row_and_back() {
    let zip_entries = sample_zip_entries();

    assert_eq!(selected_flat_index(&zip_entries, 1, 0), Some(2));
    assert_eq!(selected_row_index(&zip_entries, 1, 0), Some(4));
    assert_eq!(selection_from_flat_index(&zip_entries, 2), Some((1, 0)));
}

#[test]
fn restores_selection_from_psd_path_across_zip_boundaries() {
    let zip_entries = sample_zip_entries();

    let selection = selection_from_psd_path(&zip_entries, &PathBuf::from("b/face.psd"));

    assert_eq!(selection, Some((1, 0)));
    assert_eq!(first_psd_selection(&zip_entries), Some((0, 0)));
}

#[test]
fn library_list_state_starts_scrolling_before_selection_reaches_bottom_edge() {
    let mut app = App::loading(None);
    app.zip_entries = vec![ZipEntry {
        zip_path: PathBuf::from("assets/zip/large.zip"),
        psds: (0..10)
            .map(|index| sample_psd(&format!("large/{index}.psd"), &format!("{index}.psd")))
            .collect(),
        ..ZipEntry::default()
    }];
    app.selected_zip_index = 0;
    app.selected_psd_index = 5;

    let mut state = app.library_list_state(8);

    assert_eq!(*state.offset_mut(), 1);
}

#[test]
fn favorites_list_state_uses_same_scroll_margin_as_library_list() {
    let mut app = App::loading(None);
    app.set_favorites_for_test((0..10).map(sample_favorite).collect(), HashMap::new());
    app.toggle_favorites_view();
    for _ in 0..6 {
        app.select_next().expect("favorites selection should move");
    }

    let mut state = app.library_list_state(8);

    assert_eq!(*state.offset_mut(), 1);
}

fn sample_zip_entries() -> Vec<ZipEntry> {
    vec![
        ZipEntry {
            zip_path: PathBuf::from("assets/zip/a.zip"),
            psds: vec![
                sample_psd("a/body.psd", "body.psd"),
                sample_psd("a/mouth.psd", "mouth.psd"),
            ],
            ..ZipEntry::default()
        },
        ZipEntry {
            zip_path: PathBuf::from("assets/zip/b.zip"),
            psds: vec![sample_psd("b/face.psd", "face.psd")],
            ..ZipEntry::default()
        },
    ]
}

fn sample_psd(path: &str, file_name: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        file_name: file_name.to_string(),
        ..PsdEntry::default()
    }
}

fn sample_favorite(index: usize) -> FavoriteEntry {
    FavoriteEntry {
        zip_path: PathBuf::from(format!("assets/zip/{index}.zip")),
        psd_path_in_zip: PathBuf::from(format!("psd/{index}.psd")),
        psd_file_name: format!("{index}.psd"),
        visibility_overrides: Vec::new(),
        mascot_scale: None,
        window_position: None,
        favorite_ensemble_position: None,
    }
}
