use std::path::Path;

use anyhow::Result;
use mascot_render_core::{
    load_favorite_ensemble_enabled, mascot_config_path, set_favorite_ensemble_enabled,
};

use super::App;

impl App {
    pub(crate) fn toggle_favorite_ensemble_enabled(&mut self) -> Result<bool> {
        self.toggle_favorite_ensemble_enabled_with_config_path(&mascot_config_path(), true)
    }

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
