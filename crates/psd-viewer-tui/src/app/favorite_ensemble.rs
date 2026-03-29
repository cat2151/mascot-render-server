use std::path::Path;

use anyhow::Result;
use mascot_render_core::{
    load_favorite_ensemble_enabled, mascot_config_path, set_favorite_ensemble_enabled,
};

use super::App;

impl App {
    /// Toggles `favorite_ensemble_enabled` in mascot-render-server.toml and
    /// returns whether the current preview should be re-synced to the server.
    pub(crate) fn toggle_favorite_ensemble_enabled(&mut self) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_config_path(&mascot_config_path(), true)
    }

    /// Shared toggle implementation for production and tests.
    ///
    /// When `sync_runtime` is true, the current runtime target is re-written so
    /// mascot-render-server immediately sees the updated setting. The returned
    /// boolean indicates whether the caller should request a fresh server sync.
    fn toggle_favorite_ensemble_enabled_with_config_path(
        &mut self,
        config_path: &Path,
        sync_runtime: bool,
    ) -> Result<bool> {
        let next = !load_favorite_ensemble_enabled(config_path)?;
        set_favorite_ensemble_enabled(config_path, next)?;
        if sync_runtime {
            self.sync_current_mascot_config()?;
        }
        self.status = favorite_ensemble_status_message(next);
        Ok(true)
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
    ) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_config_path(config_path, false)
    }
}
