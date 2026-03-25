use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use mascot_render_core::{display_path, DisplayDiff, DISPLAY_DIFF_VERSION};
use mascot_render_server::{
    load_saved_window_position_for_paths, save_window_position_for_paths, SavedWindowPosition,
};

use super::{App, FocusPane};
use crate::favorites::{favorite_selection_lookup, favorites_path, save_favorites, FavoriteEntry};

const NO_SELECTED_PSD_FAVORITE_STATUS: &str = "Favorite unavailable: no PSD is selected.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FavoriteRow {
    pub(crate) label: String,
    pub(crate) available: bool,
}

impl App {
    pub(crate) fn favorites_visible(&self) -> bool {
        self.favorites_visible
    }

    pub(crate) fn favorite_rows(&self) -> Vec<FavoriteRow> {
        self.favorites
            .iter()
            .map(|favorite| {
                let zip_label = favorite
                    .zip_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| display_path(&favorite.zip_path));
                FavoriteRow {
                    label: format!(
                        "{zip_label} :: {}",
                        favorite.psd_path_in_zip.to_string_lossy()
                    ),
                    available: self.favorite_selection_lookup.contains_key(&favorite.key()),
                }
            })
            .collect()
    }

    pub(crate) fn selected_favorite_selection(&self) -> Option<usize> {
        if self.favorites.is_empty() {
            None
        } else {
            Some(self.selected_favorite_index.min(self.favorites.len() - 1))
        }
    }

    pub(crate) fn toggle_favorites_view(&mut self) {
        if self.favorites_visible {
            self.hide_favorites_view();
            self.status = "Closed favorites list.".to_string();
            return;
        }

        self.favorites_visible = true;
        self.favorites_return_focus = Some(self.focus);
        self.focus = FocusPane::Library;
        if let Some(index) = self.current_favorite_index() {
            self.selected_favorite_index = index;
        } else {
            self.sync_favorite_selection_bounds();
        }
        self.status = if self.favorites.is_empty() {
            "No favorites saved yet. Press f on the layer pane to add one.".to_string()
        } else {
            format!("Opened favorites list ({} items).", self.favorites.len())
        };
    }

    pub(crate) fn add_current_favorite(&mut self) -> Result<bool> {
        let Some((
            zip_path,
            psd_path_in_zip,
            psd_file_name,
            visibility_overrides,
            mascot_scale,
            window_position,
        )) = self
            .current_runtime_state_paths()
            .and_then(|(zip_path, psd_path_in_zip)| {
                self.selected_psd_entry().map(|psd_entry| {
                    let visibility_overrides = self
                        .variations
                        .get(&psd_entry.path)
                        .cloned()
                        .unwrap_or_default()
                        .visibility_overrides;
                    let window_position =
                        load_saved_window_position_for_paths(zip_path, psd_path_in_zip)
                            .ok()
                            .flatten()
                            .map(|position| [position.x, position.y]);
                    (
                        zip_path.to_path_buf(),
                        psd_path_in_zip.to_path_buf(),
                        psd_entry.file_name.clone(),
                        visibility_overrides,
                        self.mascot_scale,
                        window_position,
                    )
                })
            })
        else {
            self.status = NO_SELECTED_PSD_FAVORITE_STATUS.to_string();
            return Ok(false);
        };

        let favorite = FavoriteEntry {
            zip_path,
            psd_path_in_zip,
            psd_file_name,
            visibility_overrides,
            mascot_scale,
            window_position,
        };
        if let Some(index) = self
            .favorites
            .iter()
            .position(|entry| entry.key() == favorite.key())
        {
            let mut updated = false;
            if self.favorites[index].psd_file_name != favorite.psd_file_name
                || self.favorites[index].visibility_overrides != favorite.visibility_overrides
                || self.favorites[index].mascot_scale != favorite.mascot_scale
                || self.favorites[index].window_position != favorite.window_position
            {
                self.favorites[index].psd_file_name = favorite.psd_file_name.clone();
                self.favorites[index].visibility_overrides = favorite.visibility_overrides.clone();
                self.favorites[index].mascot_scale = favorite.mascot_scale;
                self.favorites[index].window_position = favorite.window_position;
                save_favorites(&favorites_path(), &self.favorites)?;
                updated = true;
            }
            self.selected_favorite_index = index;
            self.status = if updated {
                format!("Favorite updated: {}", self.favorites[index].psd_file_name)
            } else {
                format!(
                    "Favorite already saved: {}",
                    self.favorites[index].psd_file_name
                )
            };
            return Ok(updated);
        }

        self.favorites.push(favorite.clone());
        save_favorites(&favorites_path(), &self.favorites)?;
        self.selected_favorite_index = self.favorites.len().saturating_sub(1);
        self.status = format!("Favorite saved: {}", favorite.psd_file_name);
        Ok(true)
    }

    pub(crate) fn activate_selected_favorite(&mut self) -> Result<bool> {
        let Some(index) = self.selected_favorite_selection() else {
            self.status = "No favorites saved yet.".to_string();
            return Ok(false);
        };
        let favorite = self.favorites[index].clone();
        let Some((zip_index, psd_index)) =
            self.favorite_selection_lookup.get(&favorite.key()).copied()
        else {
            self.status = format!(
                "Favorite target is unavailable: {}",
                favorite.psd_path_in_zip.display()
            );
            return Ok(false);
        };

        self.selected_zip_index = zip_index;
        self.selected_psd_index = psd_index;
        if let Some(psd_path) = self.selected_psd_entry().map(|entry| entry.path.clone()) {
            apply_favorite_variation(&mut self.variations, &psd_path, &favorite);
        }
        self.selected_layer_index = 0;
        self.refresh_selected_psd_state()?;
        self.apply_favorite_mascot_scale(favorite.mascot_scale)?;
        apply_favorite_window_position(&favorite)?;
        self.hide_favorites_view();
        self.persist_workspace_state()?;
        self.status = format!("Favorite selected: {}", favorite.psd_file_name);
        Ok(true)
    }

    pub(super) fn select_previous_favorite(&mut self, step: usize) {
        let step = step.max(1);
        if let Some(current) = self.selected_favorite_selection() {
            self.selected_favorite_index = current.saturating_sub(step);
        } else if !self.favorites.is_empty() {
            self.selected_favorite_index = 0;
        }
    }

    pub(super) fn select_next_favorite(&mut self, step: usize) {
        let step = step.max(1);
        if let Some(current) = self.selected_favorite_selection() {
            let last_index = self.favorites.len().saturating_sub(1);
            self.selected_favorite_index = current.saturating_add(step).min(last_index);
        } else if !self.favorites.is_empty() {
            self.selected_favorite_index = 0;
        }
    }

    pub(super) fn sync_favorite_selection_bounds(&mut self) {
        if self.favorites.is_empty() {
            self.selected_favorite_index = 0;
            return;
        }

        self.selected_favorite_index = self
            .selected_favorite_index
            .min(self.favorites.len().saturating_sub(1));
    }

    fn current_favorite_index(&self) -> Option<usize> {
        let (zip_path, psd_path_in_zip) = self.current_runtime_state_paths()?;
        self.favorites.iter().position(|favorite| {
            favorite.zip_path == zip_path && favorite.psd_path_in_zip == psd_path_in_zip
        })
    }

    pub(super) fn rebuild_favorite_selection_lookup(&mut self) {
        self.favorite_selection_lookup = favorite_selection_lookup(&self.zip_entries);
    }

    fn hide_favorites_view(&mut self) {
        self.favorites_visible = false;
        if let Some(previous_focus) = self.favorites_return_focus.take() {
            self.focus = previous_focus;
        }
    }
}

pub(crate) fn apply_favorite_variation(
    variations: &mut HashMap<std::path::PathBuf, DisplayDiff>,
    psd_path: &Path,
    favorite: &FavoriteEntry,
) {
    let variation = DisplayDiff {
        version: DISPLAY_DIFF_VERSION,
        visibility_overrides: favorite.visibility_overrides.clone(),
    };
    if variation.is_default() {
        variations.remove(psd_path);
    } else {
        variations.insert(psd_path.to_path_buf(), variation);
    }
}

pub(crate) fn apply_favorite_window_position(favorite: &FavoriteEntry) -> Result<bool> {
    let Some([x, y]) = favorite.window_position else {
        return Ok(false);
    };
    save_window_position_for_paths(
        &favorite.zip_path,
        &favorite.psd_path_in_zip,
        SavedWindowPosition { x, y },
    )?;
    Ok(true)
}
