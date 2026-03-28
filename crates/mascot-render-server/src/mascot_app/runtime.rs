use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect};
use eframe::App;
use mascot_render_server::{captures_logical_point, TransparentHitTestUpdate};

use crate::always_bend;
use crate::eye_blink_timing::always_idle_sink_for_blink_median;

use super::{keyboard_scale_steps, scroll_scale_steps, MascotApp};

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
        self.apply_pending_restored_anchor_position(ctx);

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
        let always_idle_sink = always_idle_sink_for_blink_median(
            self.config.always_idle_sink,
            self.eye_blink.current_median_ms(),
        );
        let transform = self.motion.sample(
            now,
            self.config.bounce,
            self.config.squash_bounce,
            always_idle_sink,
        );
        let image_rect = self.window_layout.image_rect(self.base_size, transform);
        let active_skin = if blink_closed {
            self.closed_skin.as_ref().unwrap_or(&self.open_skin)
        } else {
            &self.open_skin
        };
        let texture_id = active_skin.texture.id();
        let active_image_size = active_skin.image_size;
        let active_alpha_mask = Arc::clone(&active_skin.alpha_mask);
        let bend_transform = self.config.always_bend.then(|| {
            always_bend::sample_always_bend(now - self.always_bend_started_at, image_rect)
        });
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

                if let Some(bend_transform) = bend_transform {
                    painter.add(egui::Shape::mesh(always_bend::mesh(
                        texture_id,
                        image_rect,
                        bend_transform,
                    )));
                } else {
                    painter.image(
                        texture_id,
                        image_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

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
            .repaint_after(
                now,
                self.config.bounce,
                self.config.squash_bounce,
                always_idle_sink,
            )
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
        let repaint_after = if self.config.always_bend {
            repaint_after.min(always_bend::repaint_after())
        } else {
            repaint_after
        };
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
