pub(crate) mod eye_blink;
mod favorite_ensemble;
mod favorites;
mod info;
mod layer_scroll;
pub(crate) mod library;
mod mascot_scale;
pub(crate) mod mouth_flap;
mod selection;
mod startup;
mod support;

#[cfg(test)]
pub(crate) use favorites::saved_window_positions_match_for_test;
#[cfg(test)]
pub(crate) use favorites::{apply_favorite_variation, apply_favorite_window_position};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::style::Color;

use mascot_render_core::{
    display_path, variation_png_path, variation_spec_path, Core, CoreConfig, DisplayDiff,
    PsdDocument, PsdEntry, RenderRequest, ZipEntry,
};

use crate::display_diff_state::{resolve_layer_rows, toggle_layer_override, LayerRow};
use crate::favorites::{FavoriteEntry, FavoriteKey};
use crate::tui_config::{TuiRuntimeState, DEFAULT_LAYER_SCROLL_MARGIN_RATIO};
use crate::tui_history::{save_tui_history, TuiHistory};
use crate::workspace_state::save_workspace_state;
use eye_blink::EyeBlinkAnimation;
use mouth_flap::MouthFlapAnimation;
pub(crate) use startup::{spawn_startup_loader, StartupEvent};
use support::{current_preview_status, psd_path_in_zip};

pub(crate) const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
pub(crate) const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusPane {
    Library,
    Layer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewBackend {
    MascotServer,
    Sixel,
}

#[derive(Debug)]
pub(crate) struct App {
    status: String,
    log_overlay: Option<String>,
    core: Core,
    current_psd_document: Option<PsdDocument>,
    current_preview_png_path: Option<PathBuf>,
    current_variation_spec_path: Option<PathBuf>,
    favorites_preview_png_path: Option<PathBuf>,
    processing_layer_toggle: bool,
    startup_loading: bool,
    startup_notice: Option<String>,
    preview_backend: PreviewBackend,
    terminal_focused: bool,
    help_overlay_visible: bool,
    eye_blink: Option<EyeBlinkAnimation>,
    mouth_flap: Option<MouthFlapAnimation>,
    tui_runtime_state: TuiRuntimeState,
    mascot_scale: Option<f32>,
    layer_scroll_margin_ratio: f32,
    layer_scroll_offset: usize,
    screen_height_px: Option<u16>,
    variations: HashMap<PathBuf, DisplayDiff>,
    layer_rows: Vec<LayerRow>,
    favorites: Vec<FavoriteEntry>,
    favorite_selection_lookup: HashMap<FavoriteKey, (usize, usize)>,
    favorites_visible: bool,
    favorites_return_focus: Option<FocusPane>,
    selected_favorite_index: usize,
    pub(crate) should_quit: bool,
    pub(crate) zip_entries: Vec<ZipEntry>,
    pub(crate) selected_zip_index: usize,
    pub(crate) selected_psd_index: usize,
    pub(crate) selected_layer_index: usize,
    pub(crate) focus: FocusPane,
}

impl App {
    pub(crate) fn loading(screen_height_px: Option<u16>) -> Self {
        let core = Core::new(CoreConfig::default());
        let status = format!(
            "Opening TUI first. ZIP/PSD cache will load in background from {}.",
            display_path(core.cache_dir())
        );

        Self {
            status,
            log_overlay: None,
            core,
            current_psd_document: None,
            current_preview_png_path: None,
            current_variation_spec_path: None,
            favorites_preview_png_path: None,
            processing_layer_toggle: false,
            startup_loading: true,
            startup_notice: Some("Loading ZIP/PSD cache index in background...".to_string()),
            preview_backend: PreviewBackend::MascotServer,
            terminal_focused: true,
            help_overlay_visible: false,
            eye_blink: None,
            mouth_flap: None,
            tui_runtime_state: TuiRuntimeState::default(),
            mascot_scale: None,
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
            layer_scroll_offset: 0,
            screen_height_px,
            variations: HashMap::new(),
            layer_rows: Vec::new(),
            favorites: Vec::new(),
            favorite_selection_lookup: HashMap::new(),
            favorites_visible: false,
            favorites_return_focus: None,
            selected_favorite_index: 0,
            should_quit: false,
            zip_entries: Vec::new(),
            selected_zip_index: 0,
            selected_psd_index: 0,
            selected_layer_index: 0,
            focus: FocusPane::Library,
        }
    }

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

    pub(crate) fn is_log_overlay_visible(&self) -> bool {
        self.log_overlay.is_some()
    }

    pub(crate) fn log_overlay_message(&self) -> Option<&str> {
        self.log_overlay.as_deref()
    }

    pub(crate) fn show_log_overlay(&mut self, message: impl Into<String>) {
        self.log_overlay = Some(message.into());
        self.help_overlay_visible = false;
    }

    pub(crate) fn clear_log_overlay(&mut self) {
        self.log_overlay = None;
    }

    pub(crate) fn is_startup_loading(&self) -> bool {
        self.startup_loading
    }

    pub(crate) fn startup_notice(&self) -> Option<&str> {
        self.startup_notice.as_deref()
    }

    pub(crate) fn is_terminal_focused(&self) -> bool {
        self.terminal_focused
    }

    pub(crate) fn set_terminal_focus(&mut self, focused: bool) {
        self.terminal_focused = focused;
    }

    pub(crate) fn is_help_overlay_visible(&self) -> bool {
        self.help_overlay_visible
    }

    pub(crate) fn set_help_overlay_visible(&mut self, visible: bool) {
        self.help_overlay_visible = visible;
    }

    pub(crate) fn toggle_help_overlay(&mut self) {
        self.help_overlay_visible = !self.help_overlay_visible;
    }

    pub(crate) fn processing_overlay_message(&self) -> Option<&'static str> {
        self.processing_layer_toggle.then_some("processing...")
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

    fn save_selection(&self) -> Result<()> {
        if self.selected_psd_entry().is_none() {
            return Ok(());
        }

        save_workspace_state(
            self.core.cache_dir(),
            self.selected_zip_entry()
                .map(|entry| entry.zip_hash.as_str()),
            self.selected_psd_entry().map(|entry| entry.path.as_path()),
        )
    }

    fn save_tui_history(&self) -> Result<()> {
        save_tui_history(
            self.core.cache_dir(),
            &TuiHistory {
                selected_node: self.selected_layer_selection(),
            },
        )
    }

    pub(crate) fn persist_workspace_state(&self) -> Result<()> {
        self.save_selection()?;
        self.save_tui_history()
    }

    fn current_runtime_state_paths(&self) -> Option<(&Path, &Path)> {
        let zip_path = self
            .selected_zip_entry()
            .map(|entry| entry.zip_path.as_path())?;
        let psd_path_in_zip = self
            .current_psd_document
            .as_ref()
            .map(|document| document.psd_path_in_zip.as_path())?;
        Some((zip_path, psd_path_in_zip))
    }

    fn refresh_selected_psd_state(&mut self) -> Result<()> {
        self.current_psd_document = None;
        self.current_preview_png_path = None;
        self.current_variation_spec_path = None;
        self.favorites_preview_png_path = None;
        self.clear_preview_animations();
        self.layer_rows.clear();

        let Some((zip_path, extracted_dir)) = self
            .selected_zip_entry()
            .map(|zip| (zip.zip_path.clone(), zip.extracted_dir.clone()))
        else {
            return Ok(());
        };
        let Some(psd_entry) = self.selected_psd_entry().cloned() else {
            return Ok(());
        };
        let psd_path = psd_entry.path.clone();
        let psd_path_in_zip = psd_path_in_zip(&psd_path, &extracted_dir, &psd_path);
        let document = psd_entry.to_document(&zip_path, &psd_path_in_zip);
        let variation = self.variations.entry(psd_path.clone()).or_default().clone();

        self.layer_rows = resolve_layer_rows(&document, &variation);
        self.current_psd_document = Some(document);
        self.restore_current_psd_mascot_scale()?;
        self.sync_preview_for_variation(&zip_path, &psd_path, &psd_path_in_zip, &psd_entry)?;
        let _ = self.sync_current_mascot_config()?;
        self.sync_selection_bounds();
        if self.favorites_visible {
            self.update_selected_favorite_preview();
        }
        Ok(())
    }

    fn sync_preview_for_variation(
        &mut self,
        zip_path: &Path,
        psd_path: &Path,
        psd_path_in_zip: &Path,
        psd_entry: &PsdEntry,
    ) -> Result<()> {
        let variation = self.variations.get(psd_path).cloned().unwrap_or_default();
        if variation.is_default() {
            self.current_preview_png_path = psd_entry.rendered_png_path.clone();
            self.current_variation_spec_path = None;
            return Ok(());
        }

        let rendered = self.core.render_png(RenderRequest {
            zip_path: zip_path.to_path_buf(),
            psd_path_in_zip: psd_path_in_zip.to_path_buf(),
            display_diff: variation,
        })?;
        self.current_variation_spec_path = Some(variation_spec_path(&rendered.output_path));
        self.current_preview_png_path = Some(rendered.output_path);
        Ok(())
    }
}
