use anyhow::Result;

use super::library::{
    build_library_rows, first_psd_selection, selected_flat_index, selected_row_index,
    selection_from_flat_index, selection_from_psd_path, LibraryRow,
};
use super::{App, FocusPane};
use crate::workspace_state::WorkspaceState;
use mascot_render_core::{PsdEntry, ZipEntry};

impl App {
    pub(crate) fn move_focus_left(&mut self) {
        self.focus = match self.focus {
            FocusPane::Library => FocusPane::Library,
            FocusPane::Layer => FocusPane::Library,
        };
    }

    pub(crate) fn move_focus_right(&mut self) {
        self.focus = match self.focus {
            FocusPane::Library => FocusPane::Layer,
            FocusPane::Layer => FocusPane::Layer,
        };
    }

    pub(crate) fn select_previous(&mut self) -> Result<()> {
        match self.focus {
            FocusPane::Library => {
                if let Some(current) = self.selected_flat_psd_index() {
                    if current > 0 {
                        self.select_psd_by_flat_index(current - 1)?;
                    }
                } else if let Some((zip_index, psd_index)) = first_psd_selection(&self.zip_entries)
                {
                    self.selected_zip_index = zip_index;
                    self.selected_psd_index = psd_index;
                    self.selected_layer_index = 0;
                    self.refresh_selected_psd_state()?;
                }
            }
            FocusPane::Layer => {
                if self.selected_layer_index > 0 {
                    self.selected_layer_index -= 1;
                }
            }
        }

        self.sync_selection_bounds();
        self.persist_navigation_selection()
    }

    pub(crate) fn select_next(&mut self) -> Result<()> {
        match self.focus {
            FocusPane::Library => {
                if let Some(current) = self.selected_flat_psd_index() {
                    if selection_from_flat_index(&self.zip_entries, current + 1).is_some() {
                        self.select_psd_by_flat_index(current + 1)?;
                    }
                } else if let Some((zip_index, psd_index)) = first_psd_selection(&self.zip_entries)
                {
                    self.selected_zip_index = zip_index;
                    self.selected_psd_index = psd_index;
                    self.selected_layer_index = 0;
                    self.refresh_selected_psd_state()?;
                }
            }
            FocusPane::Layer => {
                if self.selected_layer_index + 1 < self.selected_layer_rows().len() {
                    self.selected_layer_index += 1;
                }
            }
        }

        self.sync_selection_bounds();
        self.persist_navigation_selection()
    }

    pub(crate) fn page_up(&mut self, page_step: usize) -> Result<()> {
        let page_step = page_step.max(1);
        match self.focus {
            FocusPane::Library => {
                if let Some(current) = self.selected_flat_psd_index() {
                    self.select_psd_by_flat_index(current.saturating_sub(page_step))?;
                } else if let Some((zip_index, psd_index)) = first_psd_selection(&self.zip_entries)
                {
                    self.selected_zip_index = zip_index;
                    self.selected_psd_index = psd_index;
                    self.selected_layer_index = 0;
                    self.refresh_selected_psd_state()?;
                }
            }
            FocusPane::Layer => {
                self.selected_layer_index = self.selected_layer_index.saturating_sub(page_step);
            }
        }

        self.sync_selection_bounds();
        self.persist_navigation_selection()
    }

    pub(crate) fn page_down(&mut self, page_step: usize) -> Result<()> {
        let page_step = page_step.max(1);
        match self.focus {
            FocusPane::Library => {
                if let Some(current) = self.selected_flat_psd_index() {
                    let last_index = self
                        .zip_entries
                        .iter()
                        .map(|zip| zip.psds.len())
                        .sum::<usize>()
                        .saturating_sub(1);
                    let target = current.saturating_add(page_step).min(last_index);
                    if let Some((zip_index, psd_index)) =
                        selection_from_flat_index(&self.zip_entries, target)
                    {
                        self.selected_zip_index = zip_index;
                        self.selected_psd_index = psd_index;
                        self.selected_layer_index = 0;
                        self.refresh_selected_psd_state()?;
                    }
                } else if let Some((zip_index, psd_index)) = first_psd_selection(&self.zip_entries)
                {
                    self.selected_zip_index = zip_index;
                    self.selected_psd_index = psd_index;
                    self.selected_layer_index = 0;
                    self.refresh_selected_psd_state()?;
                }
            }
            FocusPane::Layer => {
                self.selected_layer_index = self.selected_layer_index.saturating_add(page_step);
            }
        }

        self.sync_selection_bounds();
        self.persist_navigation_selection()
    }

    pub(crate) fn selected_layer_selection(&self) -> Option<usize> {
        if self.layer_rows.is_empty() {
            None
        } else {
            Some(self.selected_layer_index.min(self.layer_rows.len() - 1))
        }
    }

    pub(crate) fn library_rows(&self) -> Vec<LibraryRow> {
        build_library_rows(&self.zip_entries)
    }

    pub(crate) fn selected_library_selection(&self) -> Option<usize> {
        selected_row_index(
            &self.zip_entries,
            self.selected_zip_index,
            self.selected_psd_index,
        )
    }

    pub(crate) fn selected_psds(&self) -> &[PsdEntry] {
        self.selected_zip_entry()
            .map(|zip| zip.psds.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn selected_layer_rows(&self) -> &[super::LayerRow] {
        &self.layer_rows
    }

    pub(crate) fn selected_zip_entry(&self) -> Option<&ZipEntry> {
        self.zip_entries.get(self.selected_zip_index)
    }

    pub(crate) fn selected_psd_entry(&self) -> Option<&PsdEntry> {
        self.selected_psds().get(self.selected_psd_index)
    }

    pub(crate) fn selected_layer_row(&self) -> Option<&super::LayerRow> {
        self.selected_layer_selection()
            .and_then(|index| self.layer_rows.get(index))
    }

    pub(super) fn restore_selection(&mut self, selection: WorkspaceState) {
        if let Some(psd_path) = selection.selected_psd_path.as_deref() {
            if let Some((zip_index, psd_index)) =
                selection_from_psd_path(&self.zip_entries, psd_path)
            {
                self.selected_zip_index = zip_index;
                self.selected_psd_index = psd_index;
            }
        } else if let Some(zip_hash) = selection.selected_zip_hash {
            if let Some(index) = self
                .zip_entries
                .iter()
                .position(|entry| entry.zip_hash == zip_hash)
            {
                self.selected_zip_index = index;
                self.selected_psd_index = 0;
            }
        }

        self.selected_layer_index = selection.selected_node.unwrap_or_default();
        self.sync_psd_selection_bounds();
        if !self.layer_rows.is_empty() {
            self.sync_layer_selection_bounds();
        }
    }

    pub(super) fn sync_selection_bounds(&mut self) {
        self.sync_psd_selection_bounds();
        self.sync_layer_selection_bounds();
    }

    fn sync_psd_selection_bounds(&mut self) {
        let Some((fallback_zip_index, fallback_psd_index)) = first_psd_selection(&self.zip_entries)
        else {
            self.selected_zip_index = 0;
            self.selected_psd_index = 0;
            return;
        };

        self.selected_zip_index = self.selected_zip_index.min(self.zip_entries.len() - 1);
        self.selected_psd_index = self.selected_psd_index.min(
            self.zip_entries
                .get(self.selected_zip_index)
                .map(|zip| zip.psds.len().saturating_sub(1))
                .unwrap_or_default(),
        );

        if self.selected_psd_entry().is_none() {
            self.selected_zip_index = fallback_zip_index;
            self.selected_psd_index = fallback_psd_index;
        }
    }

    fn sync_layer_selection_bounds(&mut self) {
        if self.layer_rows.is_empty() {
            self.selected_layer_index = 0;
            return;
        }

        self.selected_layer_index = self.selected_layer_index.min(self.layer_rows.len() - 1);
    }

    fn selected_flat_psd_index(&self) -> Option<usize> {
        selected_flat_index(
            &self.zip_entries,
            self.selected_zip_index,
            self.selected_psd_index,
        )
    }

    fn select_psd_by_flat_index(&mut self, flat_index: usize) -> Result<()> {
        let Some((zip_index, psd_index)) = selection_from_flat_index(&self.zip_entries, flat_index)
        else {
            return Ok(());
        };
        self.selected_zip_index = zip_index;
        self.selected_psd_index = psd_index;
        self.selected_layer_index = 0;
        self.refresh_selected_psd_state()
    }

    fn persist_navigation_selection(&self) -> Result<()> {
        match self.focus {
            FocusPane::Library => self.save_selection(),
            FocusPane::Layer => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mascot_render_core::{PsdEntry, ZipEntry};

    use super::App;
    use crate::workspace_state::WorkspaceState;

    #[test]
    fn restore_selection_keeps_layer_cursor_before_layer_rows_load() {
        let mut app = App::loading(None);
        app.zip_entries = vec![ZipEntry {
            zip_hash: "zip-a".to_string(),
            psds: vec![PsdEntry {
                path: PathBuf::from("a/body.psd"),
                file_name: "body.psd".to_string(),
                ..PsdEntry::default()
            }],
            ..ZipEntry::default()
        }];

        app.restore_selection(WorkspaceState {
            selected_psd_path: Some(PathBuf::from("a/body.psd")),
            selected_node: Some(5),
            ..WorkspaceState::default()
        });

        assert_eq!(app.selected_layer_index, 5);
    }
}

