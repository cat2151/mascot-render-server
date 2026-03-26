use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use mascot_render_core::{display_path, DisplayDiff, DISPLAY_DIFF_VERSION};
use mascot_render_server::{
    load_saved_window_position_for_paths, save_window_position_for_paths, SavedWindowPosition,
};

use super::{App, FocusPane};
use crate::favorites::{favorite_selection_lookup, favorites_path, save_favorites, FavoriteEntry};

const NO_SELECTED_PSD_FAVORITE_STATUS: &str = "Favorite unavailable: no PSD is selected.";
/// Window positions are compared in outer-window pixels; sub-half-pixel deltas are treated as unchanged.
const SAVED_WINDOW_POSITION_TOLERANCE: f32 = 0.5;

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
        self.update_selected_favorite_preview();
    }

    pub(crate) fn add_current_favorite(&mut self) -> Result<bool> {
        let Some(psd_entry) = self.selected_psd_entry() else {
            self.status = NO_SELECTED_PSD_FAVORITE_STATUS.to_string();
            return Ok(false);
        };
        let Some((zip_path, psd_path_in_zip)) = self.current_runtime_state_paths() else {
            self.status = NO_SELECTED_PSD_FAVORITE_STATUS.to_string();
            return Ok(false);
        };
        let zip_path = zip_path.to_path_buf();
        let psd_path_in_zip = psd_path_in_zip.to_path_buf();
        let psd_file_name = psd_entry.file_name.clone();
        let visibility_overrides = self
            .variations
            .get(&psd_entry.path)
            .cloned()
            .unwrap_or_default()
            .visibility_overrides;
        let window_position =
            match load_saved_window_position_for_paths(&zip_path, &psd_path_in_zip) {
                Ok(position) => position.map(|position| [position.x, position.y]),
                Err(error) => {
                    self.status = format!(
                        "Failed to add favorite: could not read saved window position ({error})."
                    );
                    return Err(error).with_context(|| {
                        format!(
                            "failed to load saved window position when adding favorite {} :: {}",
                            zip_path.display(),
                            psd_path_in_zip.display()
                        )
                    });
                }
            };
        let Some(favorite) = self.favorite_entry_from_current_state(
            zip_path,
            psd_path_in_zip,
            psd_file_name,
            visibility_overrides,
            window_position,
        ) else {
            self.status = NO_SELECTED_PSD_FAVORITE_STATUS.to_string();
            return Ok(false);
        };

        if let Some(index) = self.find_matching_favorite_index(&favorite) {
            self.selected_favorite_index = index;
            self.status = format!(
                "Favorite already saved: {}",
                self.favorites[index].psd_file_name
            );
            return Ok(false);
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
        let _ = self.apply_favorite_mascot_scale(favorite.mascot_scale)?;
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
        self.update_selected_favorite_preview();
    }

    pub(super) fn select_next_favorite(&mut self, step: usize) {
        let step = step.max(1);
        if let Some(current) = self.selected_favorite_selection() {
            let last_index = self.favorites.len().saturating_sub(1);
            self.selected_favorite_index = current.saturating_add(step).min(last_index);
        } else if !self.favorites.is_empty() {
            self.selected_favorite_index = 0;
        }
        self.update_selected_favorite_preview();
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
        let psd_file_name = self.selected_psd_entry()?.file_name.clone();
        let visibility_overrides = self
            .selected_psd_entry()
            .and_then(|psd_entry| self.variations.get(&psd_entry.path))
            .cloned()
            .unwrap_or_default()
            .visibility_overrides;
        let window_position = load_saved_window_position_for_paths(zip_path, psd_path_in_zip)
            .ok()
            .flatten()
            .map(|position| [position.x, position.y]);
        let favorite = self.favorite_entry_from_current_state(
            zip_path.to_path_buf(),
            psd_path_in_zip.to_path_buf(),
            psd_file_name,
            visibility_overrides,
            window_position,
        )?;
        self.find_matching_favorite_index(&favorite).or_else(|| {
            self.favorites.iter().position(|saved| {
                saved.zip_path == favorite.zip_path
                    && saved.psd_path_in_zip == favorite.psd_path_in_zip
            })
        })
    }

    pub(super) fn rebuild_favorite_selection_lookup(&mut self) {
        self.favorite_selection_lookup = favorite_selection_lookup(&self.zip_entries);
    }

    fn hide_favorites_view(&mut self) {
        self.favorites_visible = false;
        self.favorites_preview_png_path = None;
        if let Some(previous_focus) = self.favorites_return_focus.take() {
            self.focus = previous_focus;
        }
    }

    pub(super) fn update_selected_favorite_preview(&mut self) {
        match self.selected_favorite_preview_png_path() {
            Ok(preview_png_path) => {
                self.favorites_preview_png_path = preview_png_path;
            }
            Err(error) => {
                self.favorites_preview_png_path = None;
                self.status = format!("Favorite preview unavailable: {error}");
            }
        }
    }

    fn selected_favorite_preview_png_path(&mut self) -> Result<Option<std::path::PathBuf>> {
        let Some(index) = self.selected_favorite_selection() else {
            return Ok(None);
        };
        let favorite = &self.favorites[index];
        let Some((zip_index, psd_index)) =
            self.favorite_selection_lookup.get(&favorite.key()).copied()
        else {
            return Ok(None);
        };
        let Some(psd_entry) = self
            .zip_entries
            .get(zip_index)
            .and_then(|zip_entry| zip_entry.psds.get(psd_index))
        else {
            return Ok(None);
        };

        if favorite.visibility_overrides.is_empty() {
            return Ok(psd_entry.rendered_png_path.clone());
        }

        let rendered = self.core.render_png(mascot_render_core::RenderRequest {
            zip_path: favorite.zip_path.clone(),
            psd_path_in_zip: favorite.psd_path_in_zip.clone(),
            display_diff: DisplayDiff {
                version: DISPLAY_DIFF_VERSION,
                visibility_overrides: favorite.visibility_overrides.clone(),
            },
        })?;
        Ok(Some(rendered.output_path))
    }

    fn find_matching_favorite_index(&self, favorite: &FavoriteEntry) -> Option<usize> {
        self.favorites
            .iter()
            .position(|saved| saved.same_saved_state_as(favorite))
    }

    fn favorite_entry_from_current_state(
        &self,
        zip_path: std::path::PathBuf,
        psd_path_in_zip: std::path::PathBuf,
        psd_file_name: String,
        visibility_overrides: Vec<mascot_render_core::LayerVisibilityOverride>,
        window_position: Option<[f32; 2]>,
    ) -> Option<FavoriteEntry> {
        Some(FavoriteEntry {
            zip_path,
            psd_path_in_zip,
            psd_file_name,
            visibility_overrides,
            mascot_scale: self.mascot_scale,
            window_position,
        })
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

/// Persists the favorite's saved mascot window position and returns whether coordinates existed.
pub(crate) fn apply_favorite_window_position(favorite: &FavoriteEntry) -> Result<bool> {
    let Some([x, y]) = favorite.window_position else {
        return Ok(false);
    };

    let next_position = SavedWindowPosition { x, y };
    if let Some(saved_position) =
        load_saved_window_position_for_paths(&favorite.zip_path, &favorite.psd_path_in_zip)?
    {
        if saved_window_positions_match(saved_position, next_position) {
            return Ok(true);
        }
    }

    save_window_position_for_paths(&favorite.zip_path, &favorite.psd_path_in_zip, next_position)?;
    Ok(true)
}

/// Compares saved window positions with a small tolerance so favorites do not rewrite identical coordinates.
fn saved_window_positions_match(left: SavedWindowPosition, right: SavedWindowPosition) -> bool {
    (left.x - right.x).abs() < SAVED_WINDOW_POSITION_TOLERANCE
        && (left.y - right.y).abs() < SAVED_WINDOW_POSITION_TOLERANCE
}

#[cfg(test)]
impl App {
    pub(crate) fn set_current_preview_png_path_for_test(
        &mut self,
        path: Option<std::path::PathBuf>,
    ) {
        self.current_preview_png_path = path;
    }

    pub(crate) fn favorites_preview_png_path_for_test(&self) -> Option<&Path> {
        self.favorites_preview_png_path.as_deref()
    }

    pub(crate) fn set_favorites_for_test(
        &mut self,
        favorites: Vec<FavoriteEntry>,
        favorite_selection_lookup: HashMap<crate::favorites::FavoriteKey, (usize, usize)>,
    ) {
        self.favorites = favorites;
        self.favorite_selection_lookup = favorite_selection_lookup;
        self.sync_favorite_selection_bounds();
    }

    pub(crate) fn sync_selected_favorite_preview_for_test(&mut self) -> Result<()> {
        self.selected_favorite_preview_png_path()
            .map(|preview_png_path| {
                self.favorites_preview_png_path = preview_png_path;
            })
    }

    pub(crate) fn refresh_selected_psd_state_for_test(&mut self) -> Result<()> {
        self.refresh_selected_psd_state()
    }
}

#[cfg(test)]
mod tests {
    use super::{saved_window_positions_match, SavedWindowPosition};

    #[test]
    fn saved_window_positions_match_within_tolerance() {
        assert!(saved_window_positions_match(
            SavedWindowPosition { x: 10.0, y: 20.0 },
            SavedWindowPosition { x: 10.4, y: 20.4 }
        ));
    }

    #[test]
    fn saved_window_positions_match_rejects_boundary_and_larger_deltas() {
        assert!(!saved_window_positions_match(
            SavedWindowPosition { x: 10.0, y: 20.0 },
            SavedWindowPosition { x: 10.5, y: 20.0 }
        ));
        assert!(!saved_window_positions_match(
            SavedWindowPosition { x: 10.0, y: 20.0 },
            SavedWindowPosition { x: 10.0, y: 20.6 }
        ));
    }
}
