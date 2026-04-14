use std::path::{Path, PathBuf};

use anyhow::Result;

use mascot_render_core::{variation_png_path, variation_spec_path, PsdEntry, RenderRequest};

use crate::display_diff_state::{resolve_layer_rows, toggle_layer_override};

use super::{
    support::{current_preview_status, psd_path_in_zip},
    App, FocusPane, PreviewBackend, TimingLog,
};

impl App {
    pub(crate) fn selected_preview_png_path(&self) -> Option<&Path> {
        if self.favorites_visible {
            self.favorites_preview_png_path
                .as_deref()
                .or(self.current_preview_png_path.as_deref())
        } else {
            self.current_preview_png_path.as_deref()
        }
    }

    pub(crate) fn uses_server_preview(&self) -> bool {
        self.preview_backend == PreviewBackend::MascotServer
    }

    pub(crate) fn fallback_to_sixel_preview(&mut self, reason: String) {
        self.preview_backend = PreviewBackend::Sixel;
        self.clear_preview_animations();
        self.status = reason;
    }

    pub(crate) fn set_status_message(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub(crate) fn queue_selected_layer_toggle(&mut self) -> bool {
        if self.processing_layer_toggle
            || self.focus != FocusPane::Layer
            || self.selected_layer_selection().is_none()
            || self.selected_psd_entry().is_none()
            || self.selected_zip_entry().is_none()
            || self.current_psd_document.is_none()
        {
            return false;
        }

        self.processing_layer_toggle = true;
        true
    }

    pub(crate) fn process_pending_actions(&mut self) -> Result<bool> {
        if !self.processing_layer_toggle {
            return Ok(false);
        }

        self.processing_layer_toggle = false;
        self.toggle_selected_layer()
    }

    pub(crate) fn predicted_preview_png_path_for_selected_toggle(&self) -> Option<PathBuf> {
        if self.focus != FocusPane::Layer || self.processing_layer_toggle {
            return None;
        }

        let selected_psd_path = self.selected_psd_entry()?.path.clone();
        let document = self.current_psd_document.as_ref()?;
        let zip_entry = self.selected_zip_entry()?;
        let psd_entry = self.selected_psd_entry()?;
        let row_index = self.selected_layer_selection()?;
        let mut variation = self
            .variations
            .get(&selected_psd_path)
            .cloned()
            .unwrap_or_default();
        if !toggle_layer_override(&mut variation, document, row_index) {
            return None;
        }

        if variation.is_default() {
            return psd_entry.rendered_png_path.clone();
        }

        let psd_path_in_zip = psd_path_in_zip(
            &selected_psd_path,
            &zip_entry.extracted_dir,
            &document.psd_path_in_zip,
        );
        Some(variation_png_path(
            &zip_entry.cache_dir,
            &psd_path_in_zip,
            &psd_entry.file_name,
            &variation,
        ))
    }

    pub(crate) fn toggle_selected_layer(&mut self) -> Result<bool> {
        if self.focus != FocusPane::Layer {
            return Ok(false);
        }

        let Some(selected_psd_path) = self.selected_psd_entry().map(|entry| entry.path.clone())
        else {
            return Ok(false);
        };
        let Some(document) = self.current_psd_document.clone() else {
            return Ok(false);
        };
        let Some((zip_path, extracted_dir)) = self
            .selected_zip_entry()
            .map(|zip| (zip.zip_path.clone(), zip.extracted_dir.clone()))
        else {
            return Ok(false);
        };
        let Some(psd_entry) = self.selected_psd_entry().cloned() else {
            return Ok(false);
        };
        let psd_path_in_zip = psd_path_in_zip(
            &selected_psd_path,
            &extracted_dir,
            &document.psd_path_in_zip,
        );
        let Some(row_index) = self.selected_layer_selection() else {
            return Ok(false);
        };
        self.clear_preview_animations();

        let variation = self
            .variations
            .entry(selected_psd_path.clone())
            .or_default();
        if !toggle_layer_override(variation, &document, row_index) {
            return Ok(false);
        }

        self.layer_rows = resolve_layer_rows(&document, variation);
        self.sync_preview_for_variation(
            &zip_path,
            &selected_psd_path,
            &psd_path_in_zip,
            &psd_entry,
        )?;
        let _ = self.sync_current_mascot_config()?;
        self.status = current_preview_status(
            self.current_preview_png_path.as_deref(),
            self.current_variation_spec_path.as_deref(),
        );
        Ok(true)
    }

    pub(crate) fn sync_preview_for_variation(
        &mut self,
        zip_path: &Path,
        psd_path: &Path,
        psd_path_in_zip: &Path,
        psd_entry: &PsdEntry,
    ) -> Result<()> {
        let mut timing = TimingLog::start(
            "sync_preview_for_variation",
            format!(
                "zip_path={} psd_path={} psd_path_in_zip={}",
                zip_path.display(),
                psd_path.display(),
                psd_path_in_zip.display()
            ),
        );
        let variation = timing.measure("clone_variation", || {
            self.variations.get(psd_path).cloned().unwrap_or_default()
        });
        if variation.is_default() {
            timing.measure("set_default_preview", || {
                self.current_preview_png_path = psd_entry.rendered_png_path.clone();
                self.current_variation_spec_path = None;
            });
            return Ok(());
        }

        let rendered = timing.measure_result("Core::render_png", || {
            self.core.render_png(RenderRequest {
                zip_path: zip_path.to_path_buf(),
                psd_path_in_zip: psd_path_in_zip.to_path_buf(),
                display_diff: variation,
            })
        })?;
        timing.measure("set_variation_preview", || {
            self.current_variation_spec_path = Some(variation_spec_path(&rendered.output_path));
            self.current_preview_png_path = Some(rendered.output_path);
        });
        Ok(())
    }
}
