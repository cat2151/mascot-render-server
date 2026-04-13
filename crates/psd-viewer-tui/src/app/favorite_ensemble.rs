use std::path::Path;

use anyhow::Result;
use mascot_render_core::{
    load_favorite_ensemble_enabled, mascot_config_path, set_favorite_ensemble_enabled,
};

use super::App;
use crate::favorites::{favorites_path, load_favorites};

const FAVORITE_ENSEMBLE_EMPTY_FAVORITES_STATUS: &str =
    "Favorite ensemble unavailable: no favorites saved.";
const FAVORITE_ENSEMBLE_EMPTY_FAVORITES_MESSAGE: &str =
    "Favorite ensemble requires at least one saved favorite.\n\
No favorites are registered yet.\n\
Press f on the ZIP / PSD or layer pane to save a favorite.\n\
Press Enter or Esc to keep favorite ensemble disabled.";

impl App {
    /// Toggles `favorite_ensemble_enabled` in mascot-render-server.toml and
    /// returns whether the current preview should be re-synced to the server.
    pub(crate) fn toggle_favorite_ensemble_enabled(&mut self) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_paths(
            &mascot_config_path(),
            &favorites_path(),
            true,
        )
    }

    /// Shared toggle implementation for production and tests.
    ///
    /// When `sync_runtime` is true, the current runtime target is re-written so
    /// mascot-render-server immediately sees the updated setting. The returned
    /// boolean indicates whether a runtime sync actually happened and the caller
    /// should request a fresh server sync.
    fn toggle_favorite_ensemble_enabled_with_paths(
        &mut self,
        config_path: &Path,
        favorites_path: &Path,
        sync_runtime: bool,
    ) -> Result<bool> {
        let next = !load_favorite_ensemble_enabled(config_path)?;
        if next && load_favorites(favorites_path)?.is_empty() {
            self.status = FAVORITE_ENSEMBLE_EMPTY_FAVORITES_STATUS.to_string();
            self.show_error_overlay(FAVORITE_ENSEMBLE_EMPTY_FAVORITES_MESSAGE);
            return Ok(false);
        }
        set_favorite_ensemble_enabled(config_path, next)?;
        let runtime_synced = if sync_runtime {
            self.sync_current_mascot_config()?
        } else {
            false
        };
        self.status = favorite_ensemble_status_message(next);
        Ok(runtime_synced)
    }
}

fn favorite_ensemble_status_message(enabled: bool) -> String {
    format!("favorite_ensemble_enabled = {enabled}")
}

#[cfg(test)]
impl App {
    pub(crate) fn toggle_favorite_ensemble_enabled_for_test(
        &mut self,
        config_path: &Path,
        favorites_path: &Path,
    ) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_paths(config_path, favorites_path, false)
    }

    pub(crate) fn toggle_favorite_ensemble_enabled_with_sync_for_test(
        &mut self,
        config_path: &Path,
        favorites_path: &Path,
    ) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_paths(config_path, favorites_path, true)
    }
}
