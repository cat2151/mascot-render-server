use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use anyhow::Result;
use mascot_render_core::{display_path, existing_zip_sources, Core, CoreConfig};

use super::{App, FocusPane, PreviewBackend};
use crate::favorites::{favorites_path, load_favorites};
use crate::tui_config::{
    ensure_tui_config_split, load_tui_config, load_tui_runtime_state, save_tui_config,
    tui_config_path, TuiRuntimeState,
};
use crate::tui_history::load_tui_history;
use crate::workspace_state::{load_workspace_state, WorkspaceState};

pub(crate) enum StartupEvent {
    Progress(String),
    Snapshot(App),
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
        let result = App::load_with_progress(
            screen_height_px,
            |message| {
                let _ = progress_tx.send(StartupEvent::Progress(message));
            },
            |snapshot| {
                let _ = snapshot_tx.send(StartupEvent::Snapshot(snapshot));
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

    fn load_with_progress(
        screen_height_px: Option<u16>,
        mut progress: impl FnMut(String),
        mut snapshot_ready: impl FnMut(App),
    ) -> Result<Self> {
        let core = Core::new(CoreConfig::default());
        let tui_config_path = tui_config_path();
        let tui_config = load_tui_config(&tui_config_path)?;
        let tui_runtime_state = load_tui_runtime_state(&tui_config_path)?;
        if !tui_config_path.exists() {
            save_tui_config(&tui_config_path, &tui_config)?;
        }
        ensure_tui_config_split(&tui_config_path, &tui_config, &tui_runtime_state)?;

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
        let zip_entries = core.load_zip_entries(&zip_sources)?;

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
        self.tui_runtime_state = previous.tui_runtime_state.clone();
        self.layer_scroll_offset = previous.layer_scroll_offset;
        self.favorites = previous.favorites.clone();
        self.favorites_visible = previous.favorites_visible;
        self.favorites_return_focus = previous.favorites_return_focus;
        self.selected_favorite_index = previous.selected_favorite_index;
        let previous_selection = previous.workspace_state_snapshot();
        if previous_selection.selected_psd_path.is_some()
            || previous_selection.selected_zip_hash.is_some()
        {
            self.restore_selection(previous_selection);
        }
        self.refresh_selected_psd_state()?;
        self.persist_workspace_state()
    }

    fn workspace_state_snapshot(&self) -> WorkspaceState {
        WorkspaceState {
            selected_zip_hash: self
                .selected_zip_entry()
                .map(|entry| entry.zip_hash.clone()),
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
            core,
            current_psd_document: None,
            current_preview_png_path: None,
            current_variation_spec_path: None,
            processing_layer_toggle: false,
            startup_loading,
            startup_notice,
            preview_backend: PreviewBackend::MascotServer,
            terminal_focused: true,
            help_overlay_visible: false,
            eye_blink: None,
            mouth_flap: None,
            eye_blink_targets: tui_config.eye_blink_targets.clone(),
            tui_runtime_state,
            mascot_scale: None,
            layer_scroll_margin_ratio: tui_config.layer_scroll_margin_ratio,
            layer_scroll_offset: 0,
            screen_height_px,
            variations: HashMap::new(),
            layer_rows: Vec::new(),
            favorites,
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
