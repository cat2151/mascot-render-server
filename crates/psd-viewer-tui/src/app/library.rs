use std::path::Path;

use mascot_render_core::ZipEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryRow {
    ZipHeader { zip_index: usize },
    PsdItem { zip_index: usize, psd_index: usize },
}

pub(crate) fn build_library_rows(zip_entries: &[ZipEntry]) -> Vec<LibraryRow> {
    let mut rows = Vec::new();

    for (zip_index, zip_entry) in zip_entries.iter().enumerate() {
        rows.push(LibraryRow::ZipHeader { zip_index });
        rows.extend(
            zip_entry
                .psds
                .iter()
                .enumerate()
                .map(|(psd_index, _)| LibraryRow::PsdItem {
                    zip_index,
                    psd_index,
                }),
        );
    }

    rows
}

pub(crate) fn first_psd_selection(zip_entries: &[ZipEntry]) -> Option<(usize, usize)> {
    zip_entries
        .iter()
        .enumerate()
        .find_map(|(zip_index, zip_entry)| (!zip_entry.psds.is_empty()).then_some((zip_index, 0)))
}

pub(crate) fn selection_from_flat_index(
    zip_entries: &[ZipEntry],
    flat_index: usize,
) -> Option<(usize, usize)> {
    let mut remaining = flat_index;

    for (zip_index, zip_entry) in zip_entries.iter().enumerate() {
        if remaining < zip_entry.psds.len() {
            return Some((zip_index, remaining));
        }
        remaining = remaining.saturating_sub(zip_entry.psds.len());
    }

    None
}

pub(crate) fn selection_from_psd_path(
    zip_entries: &[ZipEntry],
    psd_path: &Path,
) -> Option<(usize, usize)> {
    zip_entries
        .iter()
        .enumerate()
        .find_map(|(zip_index, zip_entry)| {
            zip_entry
                .psds
                .iter()
                .position(|psd_entry| psd_entry.path == psd_path)
                .map(|psd_index| (zip_index, psd_index))
        })
}

pub(crate) fn selected_flat_index(
    zip_entries: &[ZipEntry],
    selected_zip_index: usize,
    selected_psd_index: usize,
) -> Option<usize> {
    let prefix = zip_entries
        .iter()
        .take(selected_zip_index)
        .map(|zip_entry| zip_entry.psds.len())
        .sum::<usize>();
    let zip_entry = zip_entries.get(selected_zip_index)?;
    (selected_psd_index < zip_entry.psds.len()).then_some(prefix + selected_psd_index)
}

pub(crate) fn selected_row_index(
    zip_entries: &[ZipEntry],
    selected_zip_index: usize,
    selected_psd_index: usize,
) -> Option<usize> {
    build_library_rows(zip_entries).iter().position(|row| {
        matches!(
            row,
            LibraryRow::PsdItem { zip_index, psd_index }
                if *zip_index == selected_zip_index && *psd_index == selected_psd_index
        )
    })
}

