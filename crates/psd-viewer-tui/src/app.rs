pub(crate) mod eye_blink;
mod favorite_ensemble;
mod favorites;
mod info;
mod layer_scroll;
pub(crate) mod library;
mod mascot_scale;
pub(crate) mod mouth_flap;
mod preview;
mod selection;
mod startup;
mod support;
mod timing;

#[cfg(test)]
pub(crate) use favorites::saved_window_positions_match_for_test;
#[cfg(test)]
pub(crate) use favorites::{apply_favorite_variation, apply_favorite_window_position};
#[cfg(test)]
pub(crate) use timing::format_timing_log_message_for_test;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::style::Color;

use mascot_render_core::{display_path, Core, CoreConfig, DisplayDiff, PsdDocument, ZipEntry};

use crate::display_diff_state::{resolve_layer_rows, LayerRow};
use crate::favorites::{FavoriteEntry, FavoriteKey};
use crate::tui_config::{TuiConfig, TuiRuntimeState, DEFAULT_LAYER_SCROLL_MARGIN_RATIO};
use crate::tui_history::{save_tui_history, TuiHistory};
use crate::workspace_state::save_workspace_state;
use eye_blink::EyeBlinkAnimation;
use mouth_flap::MouthFlapAnimation;
pub(crate) use startup::{spawn_startup_loader, StartupEvent};
use support::psd_path_in_zip;
use timing::TimingLog;

pub(crate) const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
pub(crate) const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);

#[derive(Debug, Clone)]
struct OverlayDialog {
    title: String,
    message: String,
}

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
    log_overlay: Option<OverlayDialog>,
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
    library_scroll_offset: usize,
    eye_blink_preferred_open_layer_names: Vec<String>,
    eye_blink_closed_layer_keywords: Vec<String>,
    mouth_flap_open_layer_names: Vec<String>,
    mouth_flap_closed_layer_names: Vec<String>,
    layer_scroll_offset: usize,
    screen_height_px: Option<u16>,
    variations: HashMap<PathBuf, DisplayDiff>,
    startup_pending_psd_paths: HashSet<PathBuf>,
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
        let default_tui_config = TuiConfig::default();
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
            library_scroll_offset: 0,
            eye_blink_preferred_open_layer_names: default_tui_config
                .eye_blink_preferred_open_layer_names,
            eye_blink_closed_layer_keywords: default_tui_config.eye_blink_closed_layer_keywords,
            mouth_flap_open_layer_names: default_tui_config.mouth_flap_open_layer_names,
            mouth_flap_closed_layer_names: default_tui_config.mouth_flap_closed_layer_names,
            layer_scroll_offset: 0,
            screen_height_px,
            variations: HashMap::new(),
            startup_pending_psd_paths: HashSet::new(),
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

    pub(crate) fn is_log_overlay_visible(&self) -> bool {
        self.log_overlay.is_some()
    }

    #[cfg(test)]
    pub(crate) fn log_overlay_message(&self) -> Option<&str> {
        self.log_overlay
            .as_ref()
            .map(|dialog| dialog.message.as_str())
    }

    #[cfg(test)]
    pub(crate) fn log_overlay_title(&self) -> Option<&str> {
        self.log_overlay
            .as_ref()
            .map(|dialog| dialog.title.as_str())
    }

    pub(crate) fn log_overlay_dialog(&self) -> Option<(&str, &str)> {
        self.log_overlay
            .as_ref()
            .map(|dialog| (dialog.title.as_str(), dialog.message.as_str()))
    }

    pub(crate) fn show_log_overlay(&mut self, message: impl Into<String>) {
        self.show_overlay("Log", message);
    }

    pub(crate) fn show_error_overlay(&mut self, message: impl Into<String>) {
        self.show_overlay("Error", message);
    }

    pub(crate) fn clear_log_overlay(&mut self) {
        self.log_overlay = None;
    }

    fn show_overlay(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.log_overlay = Some(OverlayDialog {
            title: title.into(),
            message: message.into(),
        });
        self.help_overlay_visible = false;
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

    fn save_selection(&self) -> Result<()> {
        if self.selected_psd_entry().is_none() {
            return Ok(());
        }

        save_workspace_state(
            self.core.cache_dir(),
            self.selected_zip_entry()
                .map(|entry| entry.zip_cache_key.as_str()),
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
        let mut timing = TimingLog::start(
            "refresh_selected_psd_state",
            self.selected_psd_timing_summary(),
        );
        timing.measure("clear_current_preview_state", || {
            self.current_psd_document = None;
            self.current_preview_png_path = None;
            self.current_variation_spec_path = None;
            self.favorites_preview_png_path = None;
            self.clear_preview_animations();
            self.layer_rows.clear();
        });

        let Some((zip_path, extracted_dir)) = self.selected_zip_entry().map(|zip| {
            timing.measure("clone_selected_zip_paths", || {
                (zip.zip_path.clone(), zip.extracted_dir.clone())
            })
        }) else {
            return Ok(());
        };
        let Some(psd_entry) = timing.measure("clone_selected_psd_entry", || {
            self.selected_psd_entry().cloned()
        }) else {
            return Ok(());
        };
        let psd_path = psd_entry.path.clone();
        if self.startup_pending_psd_paths.contains(&psd_path) {
            self.status = format!("Parsing selected PSD: {}", psd_entry.file_name);
            timing.measure("sync_selection_bounds", || self.sync_selection_bounds());
            return Ok(());
        }
        let psd_path_in_zip = psd_path_in_zip(&psd_path, &extracted_dir, &psd_path);
        let document = timing.measure("PsdEntry::to_document", || {
            psd_entry.to_document(&zip_path, &psd_path_in_zip)
        });
        let variation = self.variations.entry(psd_path.clone()).or_default().clone();

        self.layer_rows = timing.measure("resolve_layer_rows", || {
            resolve_layer_rows(&document, &variation)
        });
        self.current_psd_document = Some(document);
        timing.measure_result("restore_current_psd_mascot_scale", || {
            self.restore_current_psd_mascot_scale()
        })?;
        timing.measure_result("sync_preview_for_variation", || {
            self.sync_preview_for_variation(&zip_path, &psd_path, &psd_path_in_zip, &psd_entry)
        })?;
        let _ = timing.measure_result("sync_current_mascot_config", || {
            self.sync_current_mascot_config()
        })?;
        timing.measure("sync_selection_bounds", || self.sync_selection_bounds());
        if self.favorites_visible {
            timing.measure("update_selected_favorite_preview", || {
                self.update_selected_favorite_preview();
            });
        }
        Ok(())
    }

    fn selected_psd_timing_summary(&self) -> String {
        let zip_path = self
            .selected_zip_entry()
            .map(|entry| entry.zip_path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let psd_path = self
            .selected_psd_entry()
            .map(|entry| entry.path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "zip_index={} psd_index={} zip_path={} psd_path={}",
            self.selected_zip_index, self.selected_psd_index, zip_path, psd_path
        )
    }
}
