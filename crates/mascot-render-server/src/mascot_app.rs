use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use eframe::egui::{self, Pos2, Vec2};
use eframe::CreationContext;
use mascot_render_core::{
    load_mascot_config, load_mascot_image, mascot_runtime_state_path, Core, CoreConfig,
    MascotConfig, MascotImageData, MotionState, MotionTransform,
};
use mascot_render_server::window_history::{
    current_viewport_info, load_window_position, outer_position_for_anchor, window_history_path,
    WindowHistoryTracker,
};
use mascot_render_server::{
    anchored_inner_origin, apply_motion_timeline_request, AlphaBounds, FavoriteShufflePlaylist,
    MascotControlCommand, MascotSkinCache, MascotWindowLayout, TransparentHitTestUpdate,
    TransparentHitTestWindow,
};

use crate::app_support::{
    cached_skin_from_image, path_modified_at, size_vec, window_title, CachedSkin,
};
use crate::eye_blink::{render_closed_eye_png, EyeBlinkLoop};
use crate::favorite_gallery::{favorites_path as favorite_gallery_path, load_gallery_image};
use crate::mascot_scale::{
    adjust_scale, effective_scale, keyboard_scale_steps, persist_favorite_gallery_scale,
    persist_scale, scroll_scale_steps, SCALE_PERSIST_DEBOUNCE,
};
use crate::SKIN_CACHE_CAPACITY;
#[path = "mascot_app/runtime.rs"]
mod runtime;

pub(crate) struct MascotApp {
    config_path: PathBuf,
    runtime_state_path: PathBuf,
    config_modified_at: Option<SystemTime>,
    runtime_state_modified_at: Option<SystemTime>,
    favorite_gallery_modified_at: Option<SystemTime>,
    window_history_modified_at: Option<SystemTime>,
    config: MascotConfig,
    core: Core,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
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

pub(crate) fn allows_precise_pointer_interaction(config: &MascotConfig) -> bool {
    !config.always_bend && !config.favorite_gallery_enabled
}

pub(crate) fn transparent_hit_test_enabled(config: &MascotConfig) -> bool {
    config.transparent_background_click_through && allows_precise_pointer_interaction(config)
}

impl MascotApp {
    pub(crate) fn transparent_hit_test_enabled(&self) -> bool {
        transparent_hit_test_enabled(&self.config)
    }

    pub(crate) fn allows_precise_pointer_interaction(&self) -> bool {
        allows_precise_pointer_interaction(&self.config)
    }

    pub(crate) fn new(
        cc: &CreationContext<'_>,
        config_path: PathBuf,
        config: MascotConfig,
        image: MascotImageData,
        control_rx: Receiver<MascotControlCommand>,
        saved_window_position: Option<Pos2>,
    ) -> Self {
        let now = Instant::now();
        let scale = active_display_scale(&config, image.width, image.height);
        let base_size = size_vec(image.width, image.height, Some(scale));
        let runtime_state_path = mascot_runtime_state_path(&config_path);
        let config_modified_at = path_modified_at(&config_path);
        let runtime_state_modified_at = path_modified_at(&runtime_state_path);
        let favorite_gallery_modified_at = path_modified_at(&favorite_gallery_path());
        let open_skin = cached_skin_from_image(&cc.egui_ctx, &image);
        let initial_window_layout = MascotWindowLayout::new(
            base_size,
            open_skin.image_size,
            open_skin.content_bounds,
            config.bounce,
            config.squash_bounce,
            config.always_idle_sink,
        );
        let mut skin_cache = MascotSkinCache::new(SKIN_CACHE_CAPACITY);
        skin_cache.insert(image.path.clone(), open_skin.clone());
        let transparent_hit_test =
            TransparentHitTestWindow::try_install(cc).unwrap_or_else(|error| {
                eprintln!("transparent background click-through is disabled: {error:#}");
                TransparentHitTestWindow::disabled()
            });
        let history_path = window_history_path(&config);
        let window_history_modified_at = path_modified_at(&history_path);

        let mut app = Self {
            config_path,
            runtime_state_path,
            config_modified_at,
            runtime_state_modified_at,
            favorite_gallery_modified_at,
            window_history_modified_at,
            config,
            core: Core::new(CoreConfig::default()),
            open_skin,
            closed_skin: None,
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
            .set_always_bouncing(app.config.always_bouncing, now);
        if let Err(error) = app.refresh_closed_eye_skin(&cc.egui_ctx) {
            eprintln!("{error:#}");
        }
        app.refresh_window_layout(&cc.egui_ctx, app.window_layout);
        app.transparent_hit_test.update(TransparentHitTestUpdate {
            now: Instant::now(),
            enabled: app.transparent_hit_test_enabled(),
            debug_flash_enabled: app.config.flash_blue_background_on_transparent_input,
            alpha_mask: Arc::clone(&app.open_skin.alpha_mask),
            image_size: app.open_skin.image_size,
            image_rect: app
                .window_layout
                .image_rect(app.base_size, MotionTransform::identity()),
            pixels_per_point: cc.egui_ctx.pixels_per_point(),
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
        if self.config.favorite_gallery_enabled {
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
        self.closed_skin = None;
        self.eye_blink.reset(Instant::now());
        self.refresh_window_layout(ctx, previous_layout);
        Ok(())
    }

    fn reload_config_if_needed(&mut self, ctx: &egui::Context) -> Result<()> {
        let next_config_modified_at = path_modified_at(&self.config_path);
        let next_runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        let favorites_path = favorite_gallery_path();
        let next_favorite_gallery_modified_at = path_modified_at(&favorites_path);
        let current_history_path = window_history_path(&self.config);
        let next_window_history_modified_at = path_modified_at(&current_history_path);
        if self.config_modified_at == next_config_modified_at
            && self.runtime_state_modified_at == next_runtime_state_modified_at
            && self.favorite_gallery_modified_at == next_favorite_gallery_modified_at
            && self.window_history_modified_at == next_window_history_modified_at
        {
            return Ok(());
        }

        let previous_layout = self.window_layout;
        let next_config = load_mascot_config(&self.config_path)
            .with_context(|| format!("failed to hot-reload {}", self.config_path.display()))?;
        let favorite_gallery_changed =
            self.favorite_gallery_modified_at != next_favorite_gallery_modified_at;
        self.config_modified_at = next_config_modified_at;
        self.runtime_state_modified_at = next_runtime_state_modified_at;
        self.favorite_gallery_modified_at = next_favorite_gallery_modified_at;

        let gallery_mode_changed =
            self.config.favorite_gallery_enabled != next_config.favorite_gallery_enabled;
        let png_changed = self.config.png_path != next_config.png_path;
        let scale_changed = active_config_scale(&self.config) != active_config_scale(&next_config);
        let blink_source_changed = self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip
            || self.config.display_diff_path != next_config.display_diff_path;
        let history_path_changed = gallery_mode_changed
            || self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip;

        self.config = next_config;
        self.motion
            .set_always_bouncing(self.config.always_bouncing, Instant::now());

        if gallery_mode_changed || favorite_gallery_changed || (!self.config.favorite_gallery_enabled && png_changed) {
            self.open_skin = self.load_active_skin(ctx)?;
        }
        if gallery_mode_changed || favorite_gallery_changed || png_changed || scale_changed {
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
        if gallery_mode_changed || favorite_gallery_changed || png_changed || blink_source_changed {
            self.refresh_closed_eye_skin(ctx)?;
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
            self.restore_anchor_position(ctx, position);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title(
            &self.config,
            &self.config_path,
        )));
        Ok(())
    }

    fn load_skin(&mut self, ctx: &egui::Context, png_path: &Path) -> Result<CachedSkin> {
        if let Some(cached_skin) = self.skin_cache.get(png_path) {
            return Ok(cached_skin.clone());
        }

        let image = load_mascot_image(png_path)
            .with_context(|| format!("failed to load mascot skin {}", png_path.display()))?;
        let skin = cached_skin_from_image(ctx, &image);
        self.skin_cache.insert(png_path.to_path_buf(), skin.clone());
        Ok(skin)
    }

    fn load_active_skin(&mut self, ctx: &egui::Context) -> Result<CachedSkin> {
        if self.config.favorite_gallery_enabled {
            if let Some(image) = load_gallery_image(&self.core)? {
                return Ok(cached_skin_from_image(ctx, &image));
            }
        }
        let png_path = self.config.png_path.clone();
        self.load_skin(ctx, &png_path)
    }

    fn refresh_closed_eye_skin(&mut self, ctx: &egui::Context) -> Result<()> {
        if self.config.favorite_gallery_enabled {
            self.closed_skin = None;
            return Ok(());
        }
        self.eye_blink.reset(Instant::now());
        let Some(closed_png_path) = render_closed_eye_png(&self.core, &self.config)? else {
            self.closed_skin = None;
            return Ok(());
        };
        if closed_png_path == self.config.png_path {
            self.closed_skin = None;
            return Ok(());
        }

        self.closed_skin = Some(self.load_skin(ctx, &closed_png_path)?);
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

    fn apply_scale_steps(&mut self, ctx: &egui::Context, now: Instant, steps: i32) -> Result<()> {
        let Some(next_scale) = adjust_scale(self.scale, steps) else {
            return Ok(());
        };

        let previous_layout = self.window_layout;
        if self.config.favorite_gallery_enabled {
            self.config.favorite_gallery_scale = Some(next_scale);
        } else {
            self.config.scale = Some(next_scale);
        }
        self.scale = next_scale;
        self.pending_persisted_scale = Some(next_scale);
        self.last_scale_change_at = Some(now);
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.refresh_window_layout(ctx, previous_layout);
        ctx.request_repaint();
        Ok(())
    }

    fn pending_scale_persist_remaining(&self, now: Instant) -> Option<Duration> {
        match (self.pending_persisted_scale, self.last_scale_change_at) {
            (Some(_), Some(changed_at)) => {
                let elapsed = now.saturating_duration_since(changed_at);
                Some(SCALE_PERSIST_DEBOUNCE.saturating_sub(elapsed))
            }
            (None, None) => None,
            _ => {
                debug_assert!(
                    matches!(
                        (self.pending_persisted_scale, self.last_scale_change_at),
                        (Some(_), Some(_)) | (None, None)
                    ),
                    "pending scale debounce state should be set and cleared together"
                );
                None
            }
        }
    }

    fn persist_pending_scale_if_due(&mut self, now: Instant) -> Result<()> {
        let Some(pending_scale) = self.pending_persisted_scale else {
            return Ok(());
        };
        let pending_remaining = self.pending_scale_persist_remaining(now);
        if let Some(remaining) = pending_remaining {
            if !remaining.is_zero() {
                return Ok(());
            }
        }
        self.persist_pending_scale(pending_scale)
    }

    fn persist_pending_scale(&mut self, scale: f32) -> Result<()> {
        if self.config.favorite_gallery_enabled {
            persist_favorite_gallery_scale(&self.config_path, &self.config, scale)?;
        } else {
            persist_scale(&self.config_path, &self.config, scale)?;
            if let Err(error) = self
                .favorite_shuffle
                .persist_scale_for_current_config(&self.config, scale)
            {
                eprintln!("{error:#}");
            }
        }
        self.pending_persisted_scale = None;
        self.last_scale_change_at = None;
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        Ok(())
    }

    /// Applies the startup restore once viewport frame metrics become available.
    ///
    /// This is called every frame until `current_viewport_info` returns data so the restored
    /// anchor can be corrected by the platform-specific inner→outer offset.
    pub(crate) fn apply_pending_restored_anchor_position(&mut self, ctx: &egui::Context) {
        let Some(anchor_position) = self.pending_restored_anchor_position else {
            return;
        };
        if current_viewport_info(ctx).is_none() {
            return;
        }
        self.restore_anchor_position(ctx, anchor_position);
        self.pending_restored_anchor_position = None;
    }

    fn restore_anchor_position(&mut self, ctx: &egui::Context, anchor_position: Pos2) {
        let outer_position = current_viewport_info(ctx)
            .map(|viewport_info| {
                outer_position_for_anchor(
                    anchor_position,
                    self.window_layout.anchor_offset(),
                    viewport_info.inner_to_outer_offset,
                )
            })
            // Before viewport info is available we can only place the window using the anchor
            // offset. `apply_pending_restored_anchor_position()` re-applies the restore on a later
            // frame once the measured frame offset becomes available.
            .unwrap_or(anchor_position - self.window_layout.anchor_offset());
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
    }

    fn refresh_window_layout(&mut self, ctx: &egui::Context, previous_layout: MascotWindowLayout) {
        let viewport_info = current_viewport_info(ctx);
        let content_bounds = self.window_content_bounds();
        self.window_layout = MascotWindowLayout::new(
            self.base_size,
            self.open_skin.image_size,
            content_bounds,
            self.config.bounce,
            self.config.squash_bounce,
            self.config.always_idle_sink,
        );
        if let Some(viewport_info) = viewport_info {
            let inner_origin = anchored_inner_origin(
                viewport_info.inner_origin,
                previous_layout,
                self.window_layout,
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                inner_origin - viewport_info.inner_to_outer_offset,
            ));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
            self.window_layout.window_size(),
        ));
    }

    fn window_content_bounds(&self) -> AlphaBounds {
        let mut bounds = self.open_skin.content_bounds;
        if let Some(closed_skin) = &self.closed_skin {
            if closed_skin.image_size == self.open_skin.image_size {
                bounds = bounds.union(closed_skin.content_bounds);
            } else {
                eprintln!(
                    "closed-eye skin size {:?} does not match open skin size {:?}; using open skin bounds for the window layout",
                    closed_skin.image_size,
                    self.open_skin.image_size
                );
            }
        }
        bounds
    }
}

fn active_config_scale(config: &MascotConfig) -> Option<f32> {
    if config.favorite_gallery_enabled {
        config.favorite_gallery_scale
    } else {
        config.scale
    }
}

fn active_display_scale(config: &MascotConfig, width: u32, height: u32) -> f32 {
    if config.favorite_gallery_enabled {
        config.favorite_gallery_scale.unwrap_or(1.0)
    } else {
        effective_scale(width, height, config.scale)
    }
}
