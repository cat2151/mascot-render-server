use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use eframe::{App, CreationContext};
use mascot_render_core::{
    load_mascot_config, load_mascot_image, mascot_runtime_state_path, Core, CoreConfig,
    MascotConfig, MascotImageData, MotionState, MotionTransform,
};
use mascot_render_server::{
    anchored_inner_origin, apply_motion_timeline_request, captures_logical_point, AlphaBounds,
    FavoriteShufflePlaylist, MascotControlCommand, MascotSkinCache, MascotWindowLayout,
    TransparentHitTestUpdate, TransparentHitTestWindow,
};

use crate::app_support::{
    cached_skin_from_image, path_modified_at, size_vec, window_title, CachedSkin,
};
use crate::eye_blink::{render_closed_eye_png, EyeBlinkLoop};
use crate::mascot_scale::{
    adjust_scale, effective_scale, keyboard_scale_steps, persist_scale, scroll_scale_steps,
    SCALE_PERSIST_DEBOUNCE,
};
use crate::window_history::{
    current_viewport_info, load_window_position, window_history_path, WindowHistoryTracker,
};
use crate::SKIN_CACHE_CAPACITY;

pub(crate) struct MascotApp {
    config_path: PathBuf,
    runtime_state_path: PathBuf,
    config_modified_at: Option<SystemTime>,
    runtime_state_modified_at: Option<SystemTime>,
    config: MascotConfig,
    core: Core,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
    scale: f32,
    pending_persisted_scale: Option<f32>,
    last_scale_change_at: Option<Instant>,
    base_size: Vec2,
    skin_cache: MascotSkinCache<CachedSkin>,
    motion: MotionState,
    eye_blink: EyeBlinkLoop,
    favorite_shuffle: FavoriteShufflePlaylist,
    control_rx: Receiver<MascotControlCommand>,
    transparent_hit_test: TransparentHitTestWindow,
    window_layout: MascotWindowLayout,
    window_history: WindowHistoryTracker,
}

impl MascotApp {
    pub(crate) fn new(
        cc: &CreationContext<'_>,
        config_path: PathBuf,
        config: MascotConfig,
        image: MascotImageData,
        control_rx: Receiver<MascotControlCommand>,
        saved_window_position: Option<Pos2>,
    ) -> Self {
        let scale = effective_scale(image.width, image.height, config.scale);
        let base_size = size_vec(image.width, image.height, Some(scale));
        let runtime_state_path = mascot_runtime_state_path(&config_path);
        let config_modified_at = path_modified_at(&config_path);
        let runtime_state_modified_at = path_modified_at(&runtime_state_path);
        let open_skin = cached_skin_from_image(&cc.egui_ctx, &image);
        let initial_window_layout = MascotWindowLayout::new(
            base_size,
            open_skin.image_size,
            open_skin.content_bounds,
            config.bounce,
            config.squash_bounce,
        );
        let mut skin_cache = MascotSkinCache::new(SKIN_CACHE_CAPACITY);
        skin_cache.insert(image.path.clone(), open_skin.clone());
        let transparent_hit_test =
            TransparentHitTestWindow::try_install(cc).unwrap_or_else(|error| {
                eprintln!("transparent background click-through is disabled: {error:#}");
                TransparentHitTestWindow::disabled()
            });
        let history_path = window_history_path(&config);

        let mut app = Self {
            config_path,
            runtime_state_path,
            config_modified_at,
            runtime_state_modified_at,
            config,
            core: Core::new(CoreConfig::default()),
            open_skin,
            closed_skin: None,
            scale,
            pending_persisted_scale: None,
            last_scale_change_at: None,
            base_size,
            skin_cache,
            motion: MotionState::new(),
            eye_blink: EyeBlinkLoop::new(Instant::now()),
            favorite_shuffle: FavoriteShufflePlaylist::new(Instant::now()),
            control_rx,
            transparent_hit_test,
            window_layout: initial_window_layout,
            window_history: WindowHistoryTracker::new(history_path, saved_window_position),
        };
        app.motion
            .set_always_bouncing(app.config.always_bouncing, Instant::now());
        if let Err(error) = app.refresh_closed_eye_skin(&cc.egui_ctx) {
            eprintln!("{error:#}");
        }
        app.refresh_window_layout(&cc.egui_ctx, app.window_layout, app.base_size);
        app.transparent_hit_test.update(TransparentHitTestUpdate {
            now: Instant::now(),
            enabled: app.config.transparent_background_click_through,
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
        if self.config.png_path == png_path {
            return Ok(());
        }

        let previous_layout = self.window_layout;
        let previous_base_size = self.base_size;
        self.open_skin = self.load_skin(ctx, png_path)?;
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.config.png_path = png_path.to_path_buf();
        self.closed_skin = None;
        self.eye_blink.reset(Instant::now());
        self.refresh_window_layout(ctx, previous_layout, previous_base_size);
        Ok(())
    }

    fn reload_config_if_needed(&mut self, ctx: &egui::Context) -> Result<()> {
        let next_config_modified_at = path_modified_at(&self.config_path);
        let next_runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        if self.config_modified_at == next_config_modified_at
            && self.runtime_state_modified_at == next_runtime_state_modified_at
        {
            return Ok(());
        }

        let previous_layout = self.window_layout;
        let previous_base_size = self.base_size;
        let next_config = load_mascot_config(&self.config_path)
            .with_context(|| format!("failed to hot-reload {}", self.config_path.display()))?;
        self.config_modified_at = next_config_modified_at;
        self.runtime_state_modified_at = next_runtime_state_modified_at;

        let png_changed = self.config.png_path != next_config.png_path;
        let scale_changed = self.config.scale != next_config.scale;
        let blink_source_changed = self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip
            || self.config.display_diff_path != next_config.display_diff_path;
        let history_path_changed = self.config.zip_path != next_config.zip_path
            || self.config.psd_path_in_zip != next_config.psd_path_in_zip;

        self.config = next_config;
        self.motion
            .set_always_bouncing(self.config.always_bouncing, Instant::now());

        if png_changed {
            let next_png_path = self.config.png_path.clone();
            self.open_skin = self.load_skin(ctx, &next_png_path)?;
        }
        if png_changed || scale_changed {
            self.scale = effective_scale(
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
                self.config.scale,
            );
            self.base_size = size_vec(
                self.open_skin.image_size[0],
                self.open_skin.image_size[1],
                Some(self.scale),
            );
        }

        let mut restored_window_position = None;
        if png_changed || blink_source_changed {
            self.refresh_closed_eye_skin(ctx)?;
        }
        if history_path_changed {
            let history_path = window_history_path(&self.config);
            let saved_window_position = match load_window_position(&history_path) {
                Ok(saved_window_position) => saved_window_position,
                Err(error) => {
                    eprintln!(
                        "warning: failed to load mascot window history {}: {error:#}",
                        history_path.display()
                    );
                    None
                }
            };
            self.window_history = WindowHistoryTracker::new(history_path, saved_window_position);
            restored_window_position = saved_window_position;
        }
        self.refresh_window_layout(ctx, previous_layout, previous_base_size);
        if let Some(position) = restored_window_position {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
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

    fn refresh_closed_eye_skin(&mut self, ctx: &egui::Context) -> Result<()> {
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
            self.window_history
                .observe(viewport_info.outer_origin, now)?;
        }
        Ok(())
    }

    fn apply_scale_steps(&mut self, ctx: &egui::Context, now: Instant, steps: i32) -> Result<()> {
        let Some(next_scale) = adjust_scale(self.scale, steps) else {
            return Ok(());
        };

        let previous_layout = self.window_layout;
        let previous_base_size = self.base_size;
        self.config.scale = Some(next_scale);
        self.scale = next_scale;
        self.pending_persisted_scale = Some(next_scale);
        self.last_scale_change_at = Some(now);
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.refresh_window_layout(ctx, previous_layout, previous_base_size);
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
                    false,
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
        persist_scale(&self.config_path, &self.config, scale)?;
        self.pending_persisted_scale = None;
        self.last_scale_change_at = None;
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        Ok(())
    }

    fn refresh_window_layout(
        &mut self,
        ctx: &egui::Context,
        previous_layout: MascotWindowLayout,
        previous_base_size: Vec2,
    ) {
        let viewport_info = current_viewport_info(ctx);
        let content_bounds = self.window_content_bounds();
        self.window_layout = MascotWindowLayout::new(
            self.base_size,
            self.open_skin.image_size,
            content_bounds,
            self.config.bounce,
            self.config.squash_bounce,
        );
        if let Some(viewport_info) = viewport_info {
            let inner_origin = anchored_inner_origin(
                viewport_info.inner_origin,
                previous_layout,
                previous_base_size,
                self.window_layout,
                self.base_size,
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

impl App for MascotApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Err(error) = self.apply_control_commands(ctx) {
            eprintln!("{error:#}");
        }

        if let Err(error) = self.favorite_shuffle.update(
            &self.core,
            &self.config_path,
            &self.config,
            Instant::now(),
        ) {
            eprintln!("{error:#}");
        }

        if let Err(error) = self.reload_config_if_needed(ctx) {
            eprintln!("{error:#}");
        }

        let now = Instant::now();
        if let Err(error) = self.sync_window_history(ctx, now) {
            eprintln!("{error:#}");
        }
        if let Err(error) = self.persist_pending_scale_if_due(now) {
            eprintln!("{error:#}");
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let keyboard_steps = ctx.input(|input| {
            if !input.focused {
                return 0;
            }
            keyboard_scale_steps(
                input.modifiers,
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
                input.key_pressed(egui::Key::Minus),
            )
        });
        if let Err(error) = self.apply_scale_steps(ctx, now, keyboard_steps) {
            eprintln!("{error:#}");
        }
        let blink_closed = self.closed_skin.is_some() && self.eye_blink.is_closed(now);
        let transform = self
            .motion
            .sample(now, self.config.bounce, self.config.squash_bounce);
        let image_rect = self.window_layout.image_rect(self.base_size, transform);
        let active_skin = if blink_closed {
            self.closed_skin.as_ref().unwrap_or(&self.open_skin)
        } else {
            &self.open_skin
        };
        let texture_id = active_skin.texture.id();
        let active_image_size = active_skin.image_size;
        let active_alpha_mask = Arc::clone(&active_skin.alpha_mask);
        self.transparent_hit_test.update(TransparentHitTestUpdate {
            now,
            enabled: self.config.transparent_background_click_through,
            debug_flash_enabled: self.config.flash_blue_background_on_transparent_input,
            alpha_mask: Arc::clone(&active_alpha_mask),
            image_size: active_image_size,
            image_rect,
            pixels_per_point: ctx.pixels_per_point(),
        });
        let transparent_input_visual_remaining = self
            .transparent_hit_test
            .transparent_input_visual_remaining(now)
            .filter(|_| self.config.flash_blue_background_on_transparent_input);

        egui::Area::new("mascot-image".into())
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_size(self.window_layout.window_size());
                let (response, painter) = ui.allocate_painter(
                    self.window_layout.window_size(),
                    egui::Sense::click_and_drag(),
                );

                if transparent_input_visual_remaining.is_some() {
                    painter.rect_filled(response.rect, 0.0, Color32::from_rgb(0, 120, 255));
                }

                painter.image(
                    texture_id,
                    image_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );

                if response.clicked()
                    && response
                        .interact_pointer_pos()
                        .is_some_and(|pos| self.config.head_hitbox.contains(image_rect, pos))
                {
                    self.motion.trigger(now);
                }

                if self.config.flash_blue_background_on_transparent_input
                    && !self.config.transparent_background_click_through
                    && response.is_pointer_button_down_on()
                    && response.interact_pointer_pos().is_some_and(|pos| {
                        !captures_logical_point(
                            active_image_size,
                            image_rect,
                            active_alpha_mask.as_ref(),
                            pos,
                            8,
                        )
                    })
                {
                    self.transparent_hit_test.flash_transparent_input_visual();
                }

                if response.drag_started() || response.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                if response.secondary_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if response.hovered() {
                    let scroll_steps =
                        ctx.input(|input| scroll_scale_steps(input.raw_scroll_delta.y));
                    if let Err(error) = self.apply_scale_steps(ctx, now, scroll_steps) {
                        eprintln!("{error:#}");
                    }
                }
            });

        let repaint_after = self
            .motion
            .repaint_after(now, self.config.bounce, self.config.squash_bounce)
            .unwrap_or_else(|| {
                self.eye_blink
                    .repaint_after(now, Duration::from_millis(250))
            });
        let repaint_after = transparent_input_visual_remaining
            .map(|remaining| repaint_after.min(remaining))
            .unwrap_or(repaint_after);
        let repaint_after = self
            .pending_scale_persist_remaining(now)
            .map(|remaining| repaint_after.min(remaining))
            .unwrap_or(repaint_after);
        ctx.request_repaint_after(repaint_after);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(scale) = self.pending_persisted_scale {
            if let Err(error) = self.persist_pending_scale(scale) {
                eprintln!("{error:#}");
            }
        }
        if let Err(error) = self.window_history.flush() {
            eprintln!("{error:#}");
        }
    }
}
