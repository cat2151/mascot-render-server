use anyhow::Result;
use std::path::PathBuf;

use mascot_render_core::{
    default_mascot_scale_for_screen_height, load_mascot_image, mascot_config_path,
    mascot_window_size, write_mascot_config, MascotTarget,
};

use crate::tui_config::{save_tui_runtime_state, tui_config_path};

use super::App;

const MASCOT_SCALE_STEP: f32 = 0.10;
const MIN_MASCOT_SCALE: f32 = 0.01;

impl App {
    pub(crate) fn increase_mascot_scale(&mut self) -> Result<bool> {
        self.adjust_mascot_scale(MASCOT_SCALE_STEP, "up")
    }

    pub(crate) fn decrease_mascot_scale(&mut self) -> Result<bool> {
        self.adjust_mascot_scale(-MASCOT_SCALE_STEP, "down")
    }

    pub(crate) fn sync_current_mascot_config(&mut self) -> Result<()> {
        self.ensure_mascot_scale_initialized()?;

        let Some(png_path) = self.current_preview_png_path.clone() else {
            return Ok(());
        };
        let Some(zip_path) = self.selected_zip_entry().map(|zip| zip.zip_path.clone()) else {
            return Ok(());
        };
        let Some(psd_path_in_zip) = self
            .current_psd_document
            .as_ref()
            .map(|document| document.psd_path_in_zip.clone())
        else {
            return Ok(());
        };

        let target = MascotTarget {
            png_path,
            scale: self.mascot_scale,
            zip_path,
            psd_path_in_zip,
            display_diff_path: self.current_variation_spec_path.clone(),
        };
        write_mascot_config(&mascot_config_path(), &target)
    }

    fn adjust_mascot_scale(&mut self, delta: f32, direction: &str) -> Result<bool> {
        self.ensure_mascot_scale_initialized()?;
        let Some(current_scale) = self.mascot_scale else {
            self.status = format!(
                "Mascot scale {} ignored: preview image is not ready yet.",
                direction
            );
            return Ok(false);
        };

        let next_scale = (current_scale + delta).max(MIN_MASCOT_SCALE);
        if (next_scale - current_scale).abs() <= f32::EPSILON {
            return Ok(false);
        }

        self.persist_mascot_scale(next_scale)?;
        self.sync_current_mascot_config()?;
        self.status = format!("Mascot scale: {:.1}% of original.", next_scale * 100.0);
        Ok(true)
    }

    fn ensure_mascot_scale_initialized(&mut self) -> Result<()> {
        if self.mascot_scale.is_some() {
            return Ok(());
        }

        let Some(png_path) = self.current_preview_png_path.as_deref() else {
            return Ok(());
        };

        let image = load_mascot_image(png_path)?;
        let scale = self
            .screen_height_px
            .map(|screen_height_px| {
                default_mascot_scale_for_screen_height(image.height, screen_height_px)
            })
            .unwrap_or_else(|| legacy_scale_from_image_height(image.width, image.height));
        self.persist_mascot_scale(scale)
    }

    pub(super) fn restore_mascot_scale(&mut self, mascot_scale: Option<f32>) {
        self.mascot_scale = sanitize_scale(mascot_scale);
    }

    pub(super) fn restore_current_psd_mascot_scale(&mut self) -> Result<()> {
        let Some((zip_path, psd_path_in_zip)) =
            self.current_runtime_state_paths()
                .map(|(zip_path, psd_path_in_zip)| {
                    (zip_path.to_path_buf(), psd_path_in_zip.to_path_buf())
                })
        else {
            self.restore_mascot_scale(None);
            return Ok(());
        };

        if let Some(scale) = self
            .tui_runtime_state
            .mascot_scale_for_psd(&zip_path, &psd_path_in_zip)
        {
            self.restore_mascot_scale(Some(scale));
            return Ok(());
        }

        let legacy_scale = self.tui_runtime_state.legacy_mascot_scale;
        self.restore_mascot_scale(legacy_scale);
        if legacy_scale.is_some() {
            self.persist_psd_mascot_scale(zip_path, psd_path_in_zip, legacy_scale)?;
        }
        Ok(())
    }

    pub(super) fn apply_favorite_mascot_scale(
        &mut self,
        mascot_scale: Option<f32>,
    ) -> Result<bool> {
        let Some(scale) = sanitize_scale(mascot_scale) else {
            return Ok(false);
        };
        if self.mascot_scale == Some(scale) {
            return Ok(false);
        }

        self.persist_mascot_scale(scale)?;
        self.sync_current_mascot_config()?;
        Ok(true)
    }

    fn persist_mascot_scale(&mut self, scale: f32) -> Result<()> {
        self.mascot_scale = sanitize_scale(Some(scale));
        let Some((zip_path, psd_path_in_zip)) =
            self.current_runtime_state_paths()
                .map(|(zip_path, psd_path_in_zip)| {
                    (zip_path.to_path_buf(), psd_path_in_zip.to_path_buf())
                })
        else {
            return save_tui_runtime_state(&tui_config_path(), &self.tui_runtime_state);
        };
        self.persist_psd_mascot_scale(zip_path, psd_path_in_zip, self.mascot_scale)
    }

    fn persist_psd_mascot_scale(
        &mut self,
        zip_path: PathBuf,
        psd_path_in_zip: PathBuf,
        scale: Option<f32>,
    ) -> Result<()> {
        self.tui_runtime_state
            .set_mascot_scale_for_psd(zip_path, psd_path_in_zip, scale);
        self.tui_runtime_state.legacy_mascot_scale = None;
        save_tui_runtime_state(&tui_config_path(), &self.tui_runtime_state)
    }
}

fn sanitize_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn legacy_scale_from_image_height(width: u32, height: u32) -> f32 {
    let [_, scaled_height] = mascot_window_size(width, height, None);
    scaled_height / height.max(1) as f32
}
