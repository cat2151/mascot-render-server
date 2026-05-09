use std::time::Instant;

use anyhow::Result;
use eframe::egui::{self, Pos2, Vec2};
use mascot_render_control::log_server_info;
use mascot_render_protocol::ScreenRectPx;
use mascot_render_server::anchor_positions_from_inner_origin;
use mascot_render_server::window_history::current_viewport_info;

use super::{MascotApp, NativeWindowHandle};

impl MascotApp {
    pub(super) fn reset_window_position_if_requested(
        &mut self,
        ctx: &egui::Context,
        now: Instant,
    ) -> Result<()> {
        if !ctx.input(display_position_reset_requested) {
            return Ok(());
        }
        self.reset_window_position(ctx, now)
    }

    fn reset_window_position(&mut self, ctx: &egui::Context, now: Instant) -> Result<()> {
        let Some(outer_position) =
            reset_screen_rect(ctx, &self.native_window_handle).and_then(|screen_rect| {
                reset_outer_position_for_screen(self.window_layout.window_size(), screen_rect)
            })
        else {
            log_server_info(format!(
                "trigger=keyboard action=reset_window_position result=skipped reason=screen_rect_unavailable configured_png_path={}",
                self.config.png_path.display()
            ));
            return Ok(());
        };

        let viewport_info = current_viewport_info(ctx);
        let previous_outer_position =
            viewport_info.map(|info| info.inner_origin - info.inner_to_outer_offset);
        let target_anchor_position = viewport_info.map(|info| {
            let inner_origin = outer_position + info.inner_to_outer_offset;
            let anchor_positions =
                anchor_positions_from_inner_origin(inner_origin, self.window_layout);
            self.observe_current_placement_anchor_positions(anchor_positions);
            inner_origin + self.window_layout.anchor_offset()
        });
        if let Some(anchor_position) = target_anchor_position {
            self.window_history.observe(anchor_position, now)?;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
        ctx.request_repaint();
        log_server_info(format!(
            "trigger=keyboard action=reset_window_position result=applied previous_outer_position={} next_outer_position={} next_anchor_position={} window_size={} configured_png_path={} configured_zip_path={} configured_psd_path_in_zip={}",
            optional_pos2_text(previous_outer_position),
            pos2_text(outer_position),
            optional_pos2_text(target_anchor_position),
            vec2_text(self.window_layout.window_size()),
            self.config.png_path.display(),
            self.config.zip_path.display(),
            self.config.psd_path_in_zip.display()
        ));
        Ok(())
    }
}

fn display_position_reset_requested(input: &egui::InputState) -> bool {
    display_position_reset_requested_from_parts(
        input.focused,
        input.modifiers,
        input.key_pressed(egui::Key::R),
    )
}

fn display_position_reset_requested_from_parts(
    focused: bool,
    modifiers: egui::Modifiers,
    reset_key_pressed: bool,
) -> bool {
    focused
        && reset_key_pressed
        && !modifiers.alt
        && !modifiers.ctrl
        && !modifiers.command
        && !modifiers.mac_cmd
}

fn reset_screen_rect(
    ctx: &egui::Context,
    native_window_handle: &NativeWindowHandle,
) -> Option<ScreenRectPx> {
    ctx.input(|input| {
        input.viewport().monitor_size.map(|size| ScreenRectPx {
            min_x: 0.0,
            min_y: 0.0,
            max_x: size.x,
            max_y: size.y,
        })
    })
    .or_else(|| native_window_handle.monitor_screen_rect())
}

fn reset_outer_position_for_screen(window_size: Vec2, screen_rect: ScreenRectPx) -> Option<Pos2> {
    let x = centered_axis_position(screen_rect.min_x, screen_rect.max_x, window_size.x)?;
    let y = centered_axis_position(screen_rect.min_y, screen_rect.max_y, window_size.y)?;
    Some(Pos2::new(x, y))
}

fn centered_axis_position(min: f32, max: f32, window_size: f32) -> Option<f32> {
    if !min.is_finite() || !max.is_finite() || !window_size.is_finite() || window_size <= 0.0 {
        return None;
    }
    let screen_size = max - min;
    if screen_size <= 0.0 {
        return None;
    }
    Some(min + (screen_size - window_size).max(0.0) * 0.5)
}

fn optional_pos2_text(value: Option<Pos2>) -> String {
    value.map(pos2_text).unwrap_or_else(|| "-".to_string())
}

fn pos2_text(value: Pos2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

fn vec2_text(value: Vec2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

#[cfg(test)]
pub(crate) fn display_position_reset_requested_for_test(
    focused: bool,
    modifiers: egui::Modifiers,
    reset_key_pressed: bool,
) -> bool {
    display_position_reset_requested_from_parts(focused, modifiers, reset_key_pressed)
}

#[cfg(test)]
pub(crate) fn reset_outer_position_for_screen_for_test(
    window_size: Vec2,
    screen_rect: ScreenRectPx,
) -> Option<Pos2> {
    reset_outer_position_for_screen(window_size, screen_rect)
}
