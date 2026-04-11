use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use anyhow::Result;
use mascot_render_core::{
    display_path, existing_zip_sources, Core, CoreConfig, PsdEntry, PsdLoadProgress, ZipEntry,
    ZipLoadEvent, ZipLoadProgress,
};

use super::library::selection_from_psd_path;
use super::{App, FocusPane, PreviewBackend};
use crate::favorites::{favorites_path, load_favorites};
use crate::tui_config::{
    load_tui_config, load_tui_runtime_state, save_tui_config, tui_config_path, TuiRuntimeState,
};
use crate::tui_history::load_tui_history;
use crate::workspace_state::{load_workspace_state, WorkspaceState};

pub(crate) enum StartupEvent {
    Progress(String),
    Snapshot(App),
    Loader(ZipLoadEvent),
    Ready(Result<App>),
}

struct LoadedEntriesContext {
    restored_state: WorkspaceState,
    tui_config: crate::tui_config::TuiConfig,
    tui_runtime_state: TuiRuntimeState,
    screen_height_px: Option<u16>,
    startup_notice: Option<String>,
    status: String,
}

pub(crate) fn spawn_startup_loader(screen_height_px: Option<u16>) -> Receiver<StartupEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let progress_tx = tx.clone();
        let snapshot_tx = tx.clone();
        let loader_tx = tx.clone();
        let result = App::load_with_progress(
            screen_height_px,
            |message| {
                let _ = progress_tx.send(StartupEvent::Progress(message));
            },
            |snapshot| {
                let _ = snapshot_tx.send(StartupEvent::Snapshot(snapshot));
            },
            |event| {
                let _ = loader_tx.send(StartupEvent::Loader(event));
            },
        );
        let _ = tx.send(StartupEvent::Ready(result));
    });
    rx
}

impl App {
    pub(crate) fn apply_startup_progress(&mut self, message: String) {
        self.startup_loading = true;
        self.startup_notice = Some(message.clone());
        self.status = message;
    }

    pub(crate) fn finish_startup_error(&mut self, error: anyhow::Error) {
        self.startup_loading = false;
        self.startup_notice = Some("Startup load failed. See footer for details.".to_string());
        self.status = format!("Startup load failed: {error:#}");
    }

    pub(crate) fn apply_startup_loader_event(&mut self, event: ZipLoadEvent) -> Result<bool> {
        self.startup_loading = true;
        match event {
            ZipLoadEvent::ZipStarted(progress) => {
                self.ensure_startup_zip_entry(&progress);
                self.startup_notice =
                    Some(format!("Loading ZIP: {}", display_path(&progress.zip_path)));
                self.status = self.startup_notice.clone().unwrap_or_default();
                self.sync_selection_bounds();
                Ok(false)
            }
            ZipLoadEvent::ZipExtracted(progress) => {
                self.ensure_startup_zip_entry(&progress);
                self.startup_notice = Some(format!(
                    "Extracted ZIP: {}",
                    display_path(&progress.zip_path)
                ));
                self.status = self.startup_notice.clone().unwrap_or_default();
                Ok(false)
            }
            ZipLoadEvent::PsdDiscovered(progress) => self.apply_psd_discovered(progress),
            ZipLoadEvent::PsdReady(progress, psd) => self.apply_psd_ready(progress, *psd),
            ZipLoadEvent::ZipReady(zip_entry) => self.apply_zip_ready(zip_entry),
            ZipLoadEvent::Finished(zip_entries) => self.apply_startup_finished(zip_entries),
        }
    }

    pub(crate) fn is_psd_pending(&self, path: &Path) -> bool {
        self.startup_pending_psd_paths.contains(path)
    }

    pub(crate) fn selected_psd_is_pending(&self) -> bool {
        self.selected_psd_entry()
            .is_some_and(|entry| self.is_psd_pending(&entry.path))
    }

    fn apply_psd_discovered(&mut self, progress: PsdLoadProgress) -> Result<bool> {
        let zip_index = self.ensure_startup_zip_entry(&progress.zip);
        let psd_index = self.ensure_pending_psd_entry(zip_index, &progress);
        self.startup_pending_psd_paths
            .insert(progress.psd_path.clone());
        self.startup_notice = Some(format!("Parsing PSD: {}", progress.file_name));
        self.status = self.startup_notice.clone().unwrap_or_default();
        if self.selected_psd_entry().is_none() {
            self.selected_zip_index = zip_index;
            self.selected_psd_index = psd_index;
            self.selected_layer_index = 0;
            self.refresh_selected_psd_state()?;
        } else {
            self.sync_selection_bounds();
        }
        Ok(false)
    }

    fn apply_psd_ready(&mut self, progress: PsdLoadProgress, psd: PsdEntry) -> Result<bool> {
        let selected_path = self.selected_psd_entry().map(|entry| entry.path.clone());
        let ready_path = psd.path.clone();
        let zip_index = self.ensure_startup_zip_entry(&progress.zip);
        self.replace_or_insert_psd_entry(zip_index, psd);
        self.startup_pending_psd_paths.remove(&ready_path);
        self.status = format!("PSD ready: {}", progress.file_name);

        if selected_path.as_deref() == Some(ready_path.as_path()) {
            self.select_psd_path(&ready_path);
            self.refresh_selected_psd_state()?;
            return Ok(true);
        }

        self.sync_selection_bounds();
        Ok(false)
    }

    fn apply_zip_ready(&mut self, zip_entry: ZipEntry) -> Result<bool> {
        let selected_path = self.selected_psd_entry().map(|entry| entry.path.clone());
        let zip_path = zip_entry.zip_path.clone();
        let extracted_dir = zip_entry.extracted_dir.clone();
        self.replace_or_insert_zip_entry(zip_entry);
        self.startup_pending_psd_paths
            .retain(|path| !path.starts_with(&extracted_dir));
        self.status = format!("ZIP ready: {}", display_path(&zip_path));

        if let Some(path) = selected_path.as_deref() {
            self.select_psd_path(path);
            self.refresh_selected_psd_state()?;
            return Ok(!self.selected_psd_is_pending());
        }

        self.sync_selection_bounds();
        Ok(false)
    }

    fn apply_startup_finished(&mut self, zip_entries: Vec<ZipEntry>) -> Result<bool> {
        let selected_path = self.selected_psd_entry().map(|entry| entry.path.clone());
        self.zip_entries = zip_entries;
        self.startup_pending_psd_paths.clear();
        self.startup_loading = false;
        self.startup_notice = None;
        self.status = format!(
            "Loaded {} ZIPs / {} PSDs.",
            self.zip_entries.len(),
            self.zip_entries
                .iter()
                .map(|zip| zip.psds.len())
                .sum::<usize>()
        );

        if let Some(path) = selected_path.as_deref() {
            self.select_psd_path(path);
        }
        self.sync_selection_bounds();
        self.refresh_selected_psd_state()?;
        Ok(self.selected_psd_entry().is_some())
    }

    fn ensure_startup_zip_entry(&mut self, progress: &ZipLoadProgress) -> usize {
        if let Some(index) = self.zip_entries.iter().position(|entry| {
            entry.zip_cache_key == progress.zip_cache_key || entry.zip_path == progress.zip_path
        }) {
            return index;
        }

        self.zip_entries.push(ZipEntry {
            zip_path: progress.zip_path.clone(),
            zip_cache_key: progress.zip_cache_key.clone(),
            cache_dir: progress.cache_dir.clone(),
            extracted_dir: progress.extracted_dir.clone(),
            psd_meta_path: progress.psd_meta_path.clone(),
            psds: Vec::new(),
            updated_at: 0,
        });
        self.zip_entries.len() - 1
    }

    fn ensure_pending_psd_entry(&mut self, zip_index: usize, progress: &PsdLoadProgress) -> usize {
        let zip_entry = &mut self.zip_entries[zip_index];
        if let Some(index) = zip_entry
            .psds
            .iter()
            .position(|entry| entry.path == progress.psd_path)
        {
            return index;
        }

        zip_entry.psds.push(PsdEntry {
            path: progress.psd_path.clone(),
            file_name: progress.file_name.clone(),
            metadata: "Parsing...".to_string(),
            ..PsdEntry::default()
        });
        zip_entry.psds.len() - 1
    }

    fn replace_or_insert_psd_entry(&mut self, zip_index: usize, psd: PsdEntry) {
        let zip_entry = &mut self.zip_entries[zip_index];
        if let Some(existing) = zip_entry
            .psds
            .iter_mut()
            .find(|entry| entry.path == psd.path)
        {
            *existing = psd;
        } else {
            zip_entry.psds.push(psd);
        }
    }

    fn replace_or_insert_zip_entry(&mut self, zip_entry: ZipEntry) {
        if let Some(existing) = self.zip_entries.iter_mut().find(|entry| {
            entry.zip_cache_key == zip_entry.zip_cache_key || entry.zip_path == zip_entry.zip_path
        }) {
            *existing = zip_entry;
        } else {
            self.zip_entries.push(zip_entry);
        }
    }

    fn select_psd_path(&mut self, psd_path: &Path) {
        if let Some((zip_index, psd_index)) = selection_from_psd_path(&self.zip_entries, psd_path) {
            self.selected_zip_index = zip_index;
            self.selected_psd_index = psd_index;
        }
    }

    fn load_with_progress(
        screen_height_px: Option<u16>,
        mut progress: impl FnMut(String),
        mut snapshot_ready: impl FnMut(App),
        loader_event: impl FnMut(ZipLoadEvent),
    ) -> Result<Self> {
        let core = Core::new(CoreConfig::default());
        let tui_config_path = tui_config_path();
        let tui_config = load_tui_config(&tui_config_path)?;
        let tui_runtime_state = load_tui_runtime_state(&tui_config_path)?;
        if !tui_config_path.exists() {
            save_tui_config(&tui_config_path, &tui_config)?;
        }

        progress("Loading workspace cursor cache...".to_string());
        let mut restored_state = load_workspace_state(core.cache_dir())?;
        let tui_history = load_tui_history(core.cache_dir())?;
        restored_state.selected_node = tui_history.selected_node.or(restored_state.selected_node);

        progress("Loading cached ZIP/PSD snapshot...".to_string());
        let cached_zip_entries = core.load_cached_zip_entries_snapshot()?;
        if !cached_zip_entries.is_empty() {
            let snapshot_status = format!(
                "Loaded cached snapshot: {} ZIPs / {} PSDs. Validating source ZIPs in background...",
                cached_zip_entries.len(),
                cached_zip_entries
                    .iter()
                    .map(|zip| zip.psds.len())
                    .sum::<usize>()
            );
            let snapshot = Self::from_loaded_entries(
                core.clone(),
                cached_zip_entries,
                LoadedEntriesContext {
                    restored_state: restored_state.clone(),
                    tui_config: tui_config.clone(),
                    tui_runtime_state: tui_runtime_state.clone(),
                    screen_height_px,
                    startup_notice: Some(
                        "Validating ZIP/PSD cache against source ZIPs in background...".to_string(),
                    ),
                    status: snapshot_status,
                },
            )?;
            snapshot_ready(snapshot);
            progress("Cached ZIP/PSD snapshot is ready. Validating source ZIPs...".to_string());
        }

        progress("Loading ZIP source directories...".to_string());
        let zip_sources = existing_zip_sources();
        progress(format!(
            "Loading ZIP/PSD cache index from {} source directories...",
            zip_sources.len()
        ));
        let zip_entries = core.load_zip_entries_incremental(&zip_sources, loader_event)?;

        progress(format!(
            "Loaded {} ZIPs / {} PSDs. Preparing selected PSD state...",
            zip_entries.len(),
            zip_entries.iter().map(|zip| zip.psds.len()).sum::<usize>()
        ));
        let ready_status = format!(
            "Selections, PSD metadata, and PNG renders are cached under {}.",
            display_path(core.cache_dir())
        );
        Self::from_loaded_entries(
            core,
            zip_entries,
            LoadedEntriesContext {
                restored_state,
                tui_config,
                tui_runtime_state,
                screen_height_px,
                startup_notice: None,
                status: ready_status,
            },
        )
    }

    pub(crate) fn adopt_runtime_state_from(&mut self, previous: &App) -> Result<()> {
        self.variations = previous.variations.clone();
        self.focus = previous.focus;
        self.terminal_focused = previous.terminal_focused;
        self.help_overlay_visible = previous.help_overlay_visible;
        self.log_overlay = previous.log_overlay.clone();
        self.tui_runtime_state = previous.tui_runtime_state.clone();
        self.layer_scroll_offset = previous.layer_scroll_offset;
        self.favorites = previous.favorites.clone();
        self.rebuild_favorite_selection_lookup();
        self.favorites_visible = previous.favorites_visible;
        self.favorites_return_focus = previous.favorites_return_focus;
        self.selected_favorite_index = previous.selected_favorite_index;
        let previous_selection = previous.workspace_state_snapshot();
        if previous_selection.selected_psd_path.is_some()
            || previous_selection.selected_zip_cache_key.is_some()
        {
            self.restore_selection(previous_selection);
        }
        self.refresh_selected_psd_state()?;
        self.persist_workspace_state()
    }

    fn workspace_state_snapshot(&self) -> WorkspaceState {
        WorkspaceState {
            selected_zip_cache_key: self
                .selected_zip_entry()
                .map(|entry| entry.zip_cache_key.clone()),
            selected_psd_path: self.selected_psd_entry().map(|entry| entry.path.clone()),
            selected_node: self.selected_layer_selection(),
        }
    }

    fn from_loaded_entries(
        core: Core,
        zip_entries: Vec<mascot_render_core::ZipEntry>,
        context: LoadedEntriesContext,
    ) -> Result<Self> {
        let LoadedEntriesContext {
            restored_state,
            tui_config,
            tui_runtime_state,
            screen_height_px,
            startup_notice,
            status,
        } = context;
        let startup_loading = startup_notice.is_some();
        let favorites = load_favorites(&favorites_path())?;
        let mut app = Self {
            status,
            log_overlay: None,
            core,
            current_psd_document: None,
            current_preview_png_path: None,
            current_variation_spec_path: None,
            favorites_preview_png_path: None,
            processing_layer_toggle: false,
            startup_loading,
            startup_notice,
            preview_backend: PreviewBackend::MascotServer,
            terminal_focused: true,
            help_overlay_visible: false,
            eye_blink: None,
            mouth_flap: None,
            tui_runtime_state,
            mascot_scale: None,
            layer_scroll_margin_ratio: tui_config.layer_scroll_margin_ratio,
            eye_blink_preferred_open_layer_names: tui_config.eye_blink_preferred_open_layer_names,
            eye_blink_closed_layer_keywords: tui_config.eye_blink_closed_layer_keywords,
            mouth_flap_open_layer_names: tui_config.mouth_flap_open_layer_names,
            mouth_flap_closed_layer_names: tui_config.mouth_flap_closed_layer_names,
            layer_scroll_offset: 0,
            screen_height_px,
            variations: HashMap::new(),
            startup_pending_psd_paths: Default::default(),
            layer_rows: Vec::new(),
            favorites,
            favorite_selection_lookup: HashMap::new(),
            favorites_visible: false,
            favorites_return_focus: None,
            selected_favorite_index: 0,
            should_quit: false,
            zip_entries,
            selected_zip_index: 0,
            selected_psd_index: 0,
            selected_layer_index: 0,
            focus: FocusPane::Library,
        };

        app.rebuild_favorite_selection_lookup();
        app.restore_selection(restored_state);
        app.refresh_selected_psd_state()?;
        app.sync_favorite_selection_bounds();
        if !startup_loading {
            app.save_selection()?;
        }
        Ok(app)
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    #[test]
    fn loading_placeholder_does_not_overwrite_restored_layer_cursor() {
        let previous = App::loading(None);
        let mut loaded = App::loading(None);
        loaded.selected_layer_index = 7;

        loaded
            .adopt_runtime_state_from(&previous)
            .expect("loading placeholder should not clear restored cursor");

        assert_eq!(loaded.selected_layer_index, 7);
    }
}
