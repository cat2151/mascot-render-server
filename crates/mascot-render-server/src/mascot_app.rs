use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime};

use eframe::egui::{self, Pos2, Rect, Vec2};
use eframe::CreationContext;
use mascot_render_control::{log_server_error, MascotControlCommand};
use mascot_render_core::{
    mascot_runtime_state_path, psd_viewer_tui_activity_path, Core, CoreConfig, MascotConfig,
    MascotImageData, MotionState,
};
use mascot_render_protocol::ServerStatusStore;
use mascot_render_server::window_history::{window_history_path, WindowHistoryTracker};
use mascot_render_server::{
    load_placement_state, placement_state_path, AlphaBounds, FavoriteShufflePlaylist,
    MascotSkinCache, MascotWindowLayout, PlacementState, TransparentHitTestUpdate,
    TransparentHitTestWindow,
};

use crate::app_support::{
    cached_skin_from_image, path_modified_at, size_vec, window_title, CachedSkin,
};
use crate::ensemble::active_ensemble_path;
use crate::eye_blink::EyeBlinkLoop;
use crate::mascot_scale::{effective_scale, keyboard_scale_steps, scroll_scale_steps};
use crate::SKIN_CACHE_CAPACITY;
#[path = "mascot_app/character.rs"]
mod character;
#[path = "mascot_app/config.rs"]
mod config;
#[path = "mascot_app/context_menu.rs"]
mod context_menu;
#[path = "mascot_app/context_menu_shortcut.rs"]
mod context_menu_shortcut;
#[path = "mascot_app/control/mod.rs"]
mod control;
#[path = "mascot_app/ensemble.rs"]
mod ensemble;
#[path = "mascot_app/layout.rs"]
mod layout;
#[path = "mascot_app/logging/mod.rs"]
mod logging;
#[path = "mascot_app/mouth_flap_state.rs"]
mod mouth_flap_state;
#[path = "mascot_app/native_window.rs"]
mod native_window;
#[path = "mascot_app/persistence.rs"]
mod persistence;
#[path = "mascot_app/placement.rs"]
mod placement;
#[path = "mascot_app/reload.rs"]
mod reload;
#[path = "mascot_app/runtime.rs"]
mod runtime;
#[path = "mascot_app/scale.rs"]
mod scale;
#[path = "mascot_app/skins.rs"]
mod skins;
#[path = "mascot_app/status.rs"]
mod status;
#[path = "mascot_app/window_position_reset.rs"]
mod window_position_reset;
#[cfg(test)]
pub(crate) use character::{
    candidate_index_from_seed_for_test, character_skin_candidates_for_test,
    configured_character_name_for_status, resolve_character_skin_from_entries_for_test,
    resolve_character_skin_stably_from_entries_for_test,
};
use config::{active_display_scale, should_refresh_auxiliary_skins_now};
#[cfg(test)]
pub(crate) use config::{
    should_refresh_auxiliary_skins_now_for_test, should_reload_config_for_test,
    should_restore_window_history_for_reload_for_test,
};
#[cfg(test)]
pub(crate) use context_menu_shortcut::{
    placement_context_menu_action_for_key_for_test, PlacementContextMenuAction,
};
#[cfg(test)]
pub(crate) use control::should_consume_targeted_mouth_flap_timeline_for_test;
#[cfg(test)]
pub(crate) use ensemble::member_phase_offset_ratio;
use ensemble::EnsembleScene;
#[cfg(test)]
pub(crate) use ensemble::{member_eye_blink_elapsed, member_eye_blink_seed};
use logging::should_log_rendered_skin;
#[cfg(test)]
pub(crate) use logging::{
    change_character_failure_message_for_test, change_character_stage_message_for_test,
    change_character_success_message_for_test, clear_rendered_skin_path_for_test,
    hot_reload_context_message_for_test, record_rendered_skin_path_for_test,
    refresh_window_layout_message_for_test, reloaded_scale_message_for_test,
    rendered_skin_message_for_test, scale_change_message_for_test,
    scale_layout_change_message_for_test, should_log_rendered_skin_for_test,
    RefreshWindowLayoutDiagnosticsForTest, ScaleChangeTriggerForTest, ScaleLayoutChangeForTest,
};
#[cfg(test)]
pub(crate) use mouth_flap_state::{
    active_skin_state_for_test, mouth_flap_skin_state_for_test, ActiveSkinState,
};
use native_window::NativeWindowHandle;
#[cfg(test)]
pub(crate) use persistence::{
    persist_ensemble_mode_for_test, persist_requested_character_change_for_test,
    verify_persisted_character_change_for_test,
};
#[cfg(test)]
pub(crate) use scale::mouse_wheel_zoom_outer_position as mouse_wheel_zoom_outer_position_for_test;
#[cfg(test)]
pub(crate) use scale::pending_scale_persist_remaining_at as pending_scale_persist_remaining_at_for_test;
#[cfg(not(test))]
use status::PendingPerformanceTrace;
#[cfg(test)]
pub(crate) use status::{PendingPerformanceTrace, ServerWorkGuard};
#[cfg(test)]
pub(crate) use window_position_reset::{
    display_position_reset_requested_for_test, reset_outer_position_for_screen_for_test,
};

pub(crate) struct MascotApp {
    config_path: PathBuf,
    runtime_state_path: PathBuf,
    config_modified_at: Option<SystemTime>,
    runtime_state_modified_at: Option<SystemTime>,
    ensemble_modified_at: Option<SystemTime>,
    psd_viewer_tui_activity_modified_at: Option<SystemTime>,
    window_history_modified_at: Option<SystemTime>,
    config: MascotConfig,
    core: Core,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
    closed_skin_unavailable: bool,
    mouth_open_skin: Option<CachedSkin>,
    mouth_closed_skin: Option<CachedSkin>,
    pending_auxiliary_skin_refresh: bool,
    ensemble_scene: Option<EnsembleScene>,
    scale: f32,
    pending_persisted_scale: Option<f32>,
    last_scale_change_at: Option<Instant>,
    last_logged_skin_path: Option<PathBuf>,
    always_bend_started_at: Instant,
    base_size: Vec2,
    skin_cache: MascotSkinCache<CachedSkin>,
    motion: MotionState,
    eye_blink: EyeBlinkLoop,
    favorite_shuffle: FavoriteShufflePlaylist,
    control_rx: Receiver<MascotControlCommand>,
    transparent_hit_test: TransparentHitTestWindow,
    native_window_handle: NativeWindowHandle,
    window_layout: MascotWindowLayout,
    window_history: WindowHistoryTracker,
    pending_restored_anchor_position: Option<Pos2>,
    placement_state_path: PathBuf,
    placement_state: PlacementState,
    status_store: ServerStatusStore,
    pending_performance_traces: Vec<PendingPerformanceTrace>,
}

pub(crate) struct MascotAppStartup {
    pub(crate) control_rx: Receiver<MascotControlCommand>,
    pub(crate) saved_window_position: Option<Pos2>,
    pub(crate) status_store: ServerStatusStore,
}

pub(crate) fn click_interaction_hit_test(image_rect: Rect, pointer_pos: Pos2) -> bool {
    image_rect.contains(pointer_pos)
}

impl MascotApp {
    pub(crate) fn new(
        cc: &CreationContext<'_>,
        config_path: PathBuf,
        config: MascotConfig,
        image: MascotImageData,
        ensemble_data: Option<crate::ensemble::Ensemble>,
        startup: MascotAppStartup,
    ) -> Self {
        let MascotAppStartup {
            control_rx,
            saved_window_position,
            status_store,
        } = startup;
        let now = std::time::Instant::now();
        let scale = active_display_scale(&config, image.width, image.height);
        let runtime_state_path = mascot_runtime_state_path(&config_path);
        let config_modified_at = path_modified_at(&config_path);
        let runtime_state_modified_at = path_modified_at(&runtime_state_path);
        let ensemble_modified_at =
            active_ensemble_path(config.ensemble_mode).and_then(|path| path_modified_at(&path));
        let psd_viewer_tui_activity_modified_at =
            path_modified_at(&psd_viewer_tui_activity_path(&config_path));
        let open_skin = cached_skin_from_image(&cc.egui_ctx, &image);
        let ensemble_scene = ensemble_data.map(|ensemble| {
            EnsembleScene::from_loaded(&cc.egui_ctx, ensemble, config.always_idle_sink_enabled, now)
        });
        let base_size = ensemble_scene
            .as_ref()
            .map(|ensemble| ensemble.scaled_canvas_size(scale))
            .unwrap_or_else(|| size_vec(image.width, image.height, Some(scale)));
        let initial_window_layout = ensemble_scene
            .as_ref()
            .map(|ensemble| ensemble_window_layout(base_size, ensemble.image_size(), &config))
            .unwrap_or_else(|| {
                MascotWindowLayout::new(
                    base_size,
                    open_skin.image_size,
                    open_skin.content_bounds,
                    config.bounce,
                    config.squash_bounce,
                    config.always_idle_sink,
                )
            });
        let mut skin_cache = MascotSkinCache::new(SKIN_CACHE_CAPACITY);
        skin_cache.insert(image.path.clone(), open_skin.clone());
        let transparent_hit_test = TransparentHitTestWindow::try_install(cc)
            .expect("transparent hit test state should initialize");
        let history_path = window_history_path(&config);
        let window_history_modified_at = path_modified_at(&history_path);
        let placement_state_path = placement_state_path();
        let placement_state = match load_placement_state(&placement_state_path) {
            Ok(state) => state,
            Err(error) => {
                log_server_error(format!(
                    "failed to load mascot placement state {}: {error:#}",
                    placement_state_path.display()
                ));
                PlacementState::default()
            }
        };

        let mut app = Self {
            config_path,
            runtime_state_path,
            config_modified_at,
            runtime_state_modified_at,
            ensemble_modified_at,
            psd_viewer_tui_activity_modified_at,
            window_history_modified_at,
            config,
            core: Core::new(CoreConfig::default()),
            open_skin,
            closed_skin: None,
            closed_skin_unavailable: false,
            mouth_open_skin: None,
            mouth_closed_skin: None,
            pending_auxiliary_skin_refresh: false,
            ensemble_scene,
            scale,
            pending_persisted_scale: None,
            last_scale_change_at: None,
            last_logged_skin_path: None,
            always_bend_started_at: now,
            base_size,
            skin_cache,
            motion: MotionState::new(),
            eye_blink: EyeBlinkLoop::new(now),
            favorite_shuffle: FavoriteShufflePlaylist::new(now),
            control_rx,
            transparent_hit_test,
            native_window_handle: NativeWindowHandle::from_creation_context(cc),
            window_layout: initial_window_layout,
            window_history: WindowHistoryTracker::new(history_path, saved_window_position),
            pending_restored_anchor_position: saved_window_position,
            placement_state_path,
            placement_state,
            status_store,
            pending_performance_traces: Vec::new(),
        };
        app.initialize_current_placement_state(saved_window_position);
        app.motion
            .set_always_idle_sink_enabled(app.config.always_idle_sink_enabled, now);
        if let Some(ensemble_scene) = &mut app.ensemble_scene {
            ensemble_scene.set_always_idle_sink_enabled(app.config.always_idle_sink_enabled, now);
        }
        let _ = app.refresh_window_layout(&cc.egui_ctx, app.window_layout);
        app.transparent_hit_test.update(TransparentHitTestUpdate {
            now: Instant::now(),
        });
        app.record_lifecycle_running();
        app.refresh_status_snapshot(&cc.egui_ctx, app.config.png_path.clone(), false, None);
        app
    }

    /// Applies the startup restore once viewport frame metrics become available.
    ///
    /// This is called every frame until `current_viewport_info` returns data so the restored
    /// anchor can be corrected by the platform-specific inner→outer offset.
    pub(crate) fn apply_pending_restored_anchor_position(&mut self, ctx: &egui::Context) {
        layout::apply_pending_restored_anchor_position(self, ctx);
    }
}
fn ensemble_window_layout(
    base_size: Vec2,
    image_size: [u32; 2],
    config: &MascotConfig,
) -> MascotWindowLayout {
    MascotWindowLayout::new(
        base_size,
        image_size,
        AlphaBounds::full(image_size),
        config.bounce,
        config.squash_bounce,
        config.always_idle_sink,
    )
}
