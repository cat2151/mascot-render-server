use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use eframe::egui::{self, Pos2, Rect, Vec2};
use eframe::CreationContext;
use mascot_render_core::{
    load_mascot_config, mascot_runtime_state_path, psd_viewer_tui_activity_path, Core, CoreConfig,
    MascotConfig, MascotImageData, MotionState,
};
use mascot_render_server::window_history::{
    current_viewport_info, load_window_position, window_history_path, WindowHistoryTracker,
};
use mascot_render_server::{
    apply_motion_timeline_request, AlphaBounds, FavoriteShufflePlaylist, MascotControlCommand,
    MascotSkinCache, MascotWindowLayout, TransparentHitTestUpdate, TransparentHitTestWindow,
};

use crate::app_support::{
    cached_skin_from_image, path_modified_at, size_vec, window_title, CachedSkin,
};
use crate::eye_blink::EyeBlinkLoop;
use crate::favorite_ensemble::favorites_path as favorite_ensemble_path;
use crate::mascot_scale::{effective_scale, keyboard_scale_steps, scroll_scale_steps};
use crate::SKIN_CACHE_CAPACITY;
#[path = "mascot_app/ensemble.rs"]
mod ensemble;
#[path = "mascot_app/layout.rs"]
mod layout;
#[path = "mascot_app/runtime.rs"]
mod runtime;
#[path = "mascot_app/scale.rs"]
mod scale;
#[path = "mascot_app/skins.rs"]
mod skins;
use ensemble::FavoriteEnsembleScene;
#[cfg(test)]
pub(crate) use runtime::mouth_flap_skin_state_for_test;

const EFFECTIVE_CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
struct ReloadInputs {
    config_modified_at: Option<SystemTime>,
    runtime_state_modified_at: Option<SystemTime>,
    favorite_ensemble_modified_at: Option<SystemTime>,
    psd_viewer_tui_activity_modified_at: Option<SystemTime>,
    window_history_modified_at: Option<SystemTime>,
}

pub(crate) struct MascotApp {
    config_path: PathBuf,
    runtime_state_path: PathBuf,
    config_modified_at: Option<SystemTime>,
    runtime_state_modified_at: Option<SystemTime>,
    favorite_ensemble_modified_at: Option<SystemTime>,
    psd_viewer_tui_activity_modified_at: Option<SystemTime>,
    window_history_modified_at: Option<SystemTime>,
    last_effective_config_check_at: Instant,
    config: MascotConfig,
    core: Core,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
    mouth_open_skin: Option<CachedSkin>,
    mouth_closed_skin: Option<CachedSkin>,
    favorite_ensemble: Option<FavoriteEnsembleScene>,
    scale: f32,
    pending_persisted_scale: Option<f32>,
    last_scale_change_at: Option<Instant>,
    always_bend_started_at: Instant,
    base_size: Vec2,
    skin_cache: MascotSkinCache<CachedSkin>,
    motion: MotionState,
    eye_blink: EyeBlinkLoop,
    favorite_shuffle: FavoriteShufflePlaylist,
    control_rx: Receiver<MascotControlCommand>,
    transparent_hit_test: TransparentHitTestWindow,
    window_layout: MascotWindowLayout,
    window_history: WindowHistoryTracker,
    pending_restored_anchor_position: Option<Pos2>,
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
        favorite_ensemble_data: Option<crate::favorite_ensemble::FavoriteEnsemble>,
        control_rx: Receiver<MascotControlCommand>,
        saved_window_position: Option<Pos2>,
    ) -> Self {
        let now = Instant::now();
        let scale = active_display_scale(&config, image.width, image.height);
        let runtime_state_path = mascot_runtime_state_path(&config_path);
        let config_modified_at = path_modified_at(&config_path);
        let runtime_state_modified_at = path_modified_at(&runtime_state_path);
        let favorite_ensemble_modified_at = path_modified_at(&favorite_ensemble_path());
        let psd_viewer_tui_activity_modified_at =
            path_modified_at(&psd_viewer_tui_activity_path(&config_path));
        let open_skin = cached_skin_from_image(&cc.egui_ctx, &image);
        let favorite_ensemble = favorite_ensemble_data.map(|ensemble| {
            FavoriteEnsembleScene::from_loaded(
                &cc.egui_ctx,
                ensemble,
                config.always_idle_sink_enabled,
                now,
            )
        });
        let base_size = favorite_ensemble
            .as_ref()
            .map(|ensemble| ensemble.scaled_canvas_size(scale))
            .unwrap_or_else(|| size_vec(image.width, image.height, Some(scale)));
        let initial_window_layout = favorite_ensemble
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

        let mut app = Self {
            config_path,
            runtime_state_path,
            config_modified_at,
            runtime_state_modified_at,
            favorite_ensemble_modified_at,
            psd_viewer_tui_activity_modified_at,
            window_history_modified_at,
            last_effective_config_check_at: now,
            config,
            core: Core::new(CoreConfig::default()),
            open_skin,
            closed_skin: None,
            mouth_open_skin: None,
            mouth_closed_skin: None,
            favorite_ensemble,
            scale,
            pending_persisted_scale: None,
            last_scale_change_at: None,
            always_bend_started_at: now,
            base_size,
            skin_cache,
            motion: MotionState::new(),
            eye_blink: EyeBlinkLoop::new(now),
            favorite_shuffle: FavoriteShufflePlaylist::new(now),
            control_rx,
            transparent_hit_test,
            window_layout: initial_window_layout,
            window_history: WindowHistoryTracker::new(history_path, saved_window_position),
            pending_restored_anchor_position: saved_window_position,
        };
        app.motion
            .set_always_idle_sink_enabled(app.config.always_idle_sink_enabled, now);
        if let Some(favorite_ensemble) = &mut app.favorite_ensemble {
            favorite_ensemble
                .set_always_idle_sink_enabled(app.config.always_idle_sink_enabled, now);
        }
        if let Err(error) = app.refresh_closed_eye_skin(&cc.egui_ctx) {
            eprintln!("{error:#}");
        }
        if let Err(error) = app.refresh_mouth_flap_skins(&cc.egui_ctx) {
            eprintln!("{error:#}");
        }
        app.refresh_window_layout(&cc.egui_ctx, app.window_layout);
        app.transparent_hit_test.update(TransparentHitTestUpdate {
            now: Instant::now(),
        });
        app
    }

    fn apply_control_commands(&mut self, ctx: &egui::Context) -> Result<()> {
        while let Ok(command) = self.control_rx.try_recv() {
            match command {
                MascotControlCommand::Show => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                }
                MascotControlCommand::Hide => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                MascotControlCommand::ChangeSkin(png_path) => {
                    self.change_skin(ctx, &png_path)?;
                }
                MascotControlCommand::PlayTimeline(request) => {
                    apply_motion_timeline_request(
                        &mut self.motion,
                        self.window_layout,
                        Instant::now(),
                        request,
                    )?;
                }
            }
            ctx.request_repaint();
        }

        Ok(())
    }

    fn change_skin(&mut self, ctx: &egui::Context, png_path: &Path) -> Result<()> {
        if self.config.favorite_ensemble_enabled {
            return Ok(());
        }
        if self.config.png_path == png_path {
            return Ok(());
        }

        let previous_layout = self.window_layout;
        self.open_skin = self.load_skin(ctx, png_path)?;
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.config.png_path = png_path.to_path_buf();
        self.eye_blink.reset(Instant::now());
        self.refresh_closed_eye_skin(ctx)?;
        self.refresh_mouth_flap_skins(ctx)?;
        self.refresh_window_layout(ctx, previous_layout);
        Ok(())
    }

    fn reload_config_if_needed(&mut self, ctx: &egui::Context) -> Result<()> {
        let now = Instant::now();
        let next_config_modified_at = path_modified_at(&self.config_path);
        let next_runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        let favorites_path = favorite_ensemble_path();
        let next_favorite_ensemble_modified_at = path_modified_at(&favorites_path);
        let next_psd_viewer_tui_activity_modified_at =
            path_modified_at(&psd_viewer_tui_activity_path(&self.config_path));
        let current_history_path = window_history_path(&self.config);
        let next_window_history_modified_at = path_modified_at(&current_history_path);
        if !should_reload_config(
            ReloadInputs {
                config_modified_at: self.config_modified_at,
                runtime_state_modified_at: self.runtime_state_modified_at,
                favorite_ensemble_modified_at: self.favorite_ensemble_modified_at,
                psd_viewer_tui_activity_modified_at: self.psd_viewer_tui_activity_modified_at,
                window_history_modified_at: self.window_history_modified_at,
            },
            ReloadInputs {
                config_modified_at: next_config_modified_at,
                runtime_state_modified_at: next_runtime_state_modified_at,
                favorite_ensemble_modified_at: next_favorite_ensemble_modified_at,
                psd_viewer_tui_activity_modified_at: next_psd_viewer_tui_activity_modified_at,
                window_history_modified_at: next_window_history_modified_at,
            },
            self.last_effective_config_check_at,
            now,
        ) {
            return Ok(());
        }

        let previous_layout = self.window_layout;
        let next_config = load_mascot_config(&self.config_path)
            .with_context(|| format!("failed to hot-reload {}", self.config_path.display()))?;
        let favorite_ensemble_changed =
            self.favorite_ensemble_modified_at != next_favorite_ensemble_modified_at;
        self.config_modified_at = next_config_modified_at;
        self.runtime_state_modified_at = next_runtime_state_modified_at;
        self.favorite_ensemble_modified_at = next_favorite_ensemble_modified_at;
        self.psd_viewer_tui_activity_modified_at = next_psd_viewer_tui_activity_modified_at;
        self.last_effective_config_check_at = now;

        let ensemble_mode_changed =
            self.config.favorite_ensemble_enabled != next_config.favorite_ensemble_enabled;
        let png_changed = self.config.png_path != next_config.png_path;
        let scale_changed = active_config_scale(&self.config) != active_config_scale(&next_config);
        let blink_source_changed = self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip
            || self.config.display_diff_path != next_config.display_diff_path;
        let history_path_changed = ensemble_mode_changed
            || self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip;

        self.config = next_config;
        self.motion
            .set_always_idle_sink_enabled(self.config.always_idle_sink_enabled, Instant::now());
        if let Some(favorite_ensemble) = &mut self.favorite_ensemble {
            favorite_ensemble
                .set_always_idle_sink_enabled(self.config.always_idle_sink_enabled, Instant::now());
        }
        if png_changed || ensemble_mode_changed {
            self.open_skin = self.load_active_skin(ctx)?;
        }

        if self.config.favorite_ensemble_enabled {
            if ensemble_mode_changed || favorite_ensemble_changed {
                self.favorite_ensemble = self.load_active_ensemble_scene(ctx)?;
            }
        } else if ensemble_mode_changed || png_changed {
            self.favorite_ensemble = None;
        }
        if ensemble_mode_changed || favorite_ensemble_changed || png_changed || scale_changed {
            self.scale = active_display_scale(
                &self.config,
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
            );
            self.base_size = size_vec(
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
                Some(self.scale),
            );
        }

        let mut restored_window_position = None;
        if ensemble_mode_changed || favorite_ensemble_changed || png_changed || blink_source_changed
        {
            self.refresh_closed_eye_skin(ctx)?;
            self.refresh_mouth_flap_skins(ctx)?;
        }
        if history_path_changed
            || self.window_history_modified_at != next_window_history_modified_at
        {
            let next_history_path = if history_path_changed {
                window_history_path(&self.config)
            } else {
                current_history_path
            };
            let saved_window_position = match load_window_position(&next_history_path) {
                Ok(saved_window_position) => saved_window_position,
                Err(error) => {
                    eprintln!(
                        "warning: failed to load mascot window history {}: {error:#}",
                        next_history_path.display()
                    );
                    None
                }
            };
            self.window_history_modified_at = path_modified_at(&next_history_path);
            self.window_history =
                WindowHistoryTracker::new(next_history_path, saved_window_position);
            restored_window_position = saved_window_position;
        }
        self.refresh_window_layout(ctx, previous_layout);
        if let Some(position) = restored_window_position {
            layout::restore_anchor_position(self, ctx, position);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title(
            &self.config,
            &self.config_path,
        )));
        Ok(())
    }
    fn sync_window_history(&mut self, ctx: &egui::Context, now: Instant) -> Result<()> {
        if let Some(viewport_info) = current_viewport_info(ctx) {
            self.window_history.observe(
                viewport_info.inner_origin + self.window_layout.anchor_offset(),
                now,
            )?;
            self.window_history_modified_at = path_modified_at(self.window_history.path());
        }
        Ok(())
    }

    /// Applies the startup restore once viewport frame metrics become available.
    ///
    /// This is called every frame until `current_viewport_info` returns data so the restored
    /// anchor can be corrected by the platform-specific inner→outer offset.
    pub(crate) fn apply_pending_restored_anchor_position(&mut self, ctx: &egui::Context) {
        layout::apply_pending_restored_anchor_position(self, ctx);
    }
}

fn active_config_scale(config: &MascotConfig) -> Option<f32> {
    if config.favorite_ensemble_enabled {
        config.favorite_ensemble_scale
    } else {
        config.scale
    }
}

fn active_display_scale(config: &MascotConfig, width: u32, height: u32) -> f32 {
    if config.favorite_ensemble_enabled {
        config.favorite_ensemble_scale.unwrap_or(1.0)
    } else {
        effective_scale(width, height, config.scale)
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

fn should_reload_config(
    current: ReloadInputs,
    next: ReloadInputs,
    last_effective_config_check_at: Instant,
    now: Instant,
) -> bool {
    current.config_modified_at != next.config_modified_at
        || current.runtime_state_modified_at != next.runtime_state_modified_at
        || current.favorite_ensemble_modified_at != next.favorite_ensemble_modified_at
        || current.psd_viewer_tui_activity_modified_at != next.psd_viewer_tui_activity_modified_at
        || current.window_history_modified_at != next.window_history_modified_at
        || now.duration_since(last_effective_config_check_at) >= EFFECTIVE_CONFIG_POLL_INTERVAL
}

#[cfg(test)]
pub(crate) fn should_reload_config_for_test(
    current: [Option<SystemTime>; 5],
    next: [Option<SystemTime>; 5],
    last_effective_config_check_at: Instant,
    now: Instant,
) -> bool {
    should_reload_config(
        ReloadInputs {
            config_modified_at: current[0],
            runtime_state_modified_at: current[1],
            favorite_ensemble_modified_at: current[2],
            psd_viewer_tui_activity_modified_at: current[3],
            window_history_modified_at: current[4],
        },
        ReloadInputs {
            config_modified_at: next[0],
            runtime_state_modified_at: next[1],
            favorite_ensemble_modified_at: next[2],
            psd_viewer_tui_activity_modified_at: next[3],
            window_history_modified_at: next[4],
        },
        last_effective_config_check_at,
        now,
    )
}
