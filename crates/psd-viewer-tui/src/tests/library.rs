use std::path::PathBuf;

use mascot_render_core::{PsdEntry, ZipEntry};

use crate::app::library::{
    build_library_rows, first_psd_selection, selected_flat_index, selected_row_index,
    selection_from_flat_index, selection_from_psd_path, LibraryRow,
};

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
