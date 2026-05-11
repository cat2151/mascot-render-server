use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;
use mascot_render_protocol::{PlacementAnchorKind, ScreenRectPx};

use super::logging::ScaleChangeTrigger;
use super::placement::screen_safe_rect;
use super::MascotApp;
use crate::app_support::{path_modified_at, size_vec};
use crate::mascot_scale::{
    adjust_scale, persist_ensemble_scale, persist_scale, SCALE_PERSIST_DEBOUNCE,
};
use mascot_render_server::window_history::{current_viewport_info, ViewportInfo};
use mascot_render_server::{clamp_zoomed_inner_origin_to_right_edge, MascotWindowLayout};

impl MascotApp {
    pub(super) fn apply_scale_steps(
        &mut self,
        ctx: &egui::Context,
        now: Instant,
        steps: i32,
        trigger: ScaleChangeTrigger,
    ) -> Result<()> {
        let Some(next_scale) = adjust_scale(self.scale, steps) else {
            return Ok(());
        };

        let previous_scale = self.scale;
        let previous_layout = self.window_layout;
        let previous_viewport_info = current_viewport_info(ctx);
        if self.config.ensemble_mode.is_ensemble() {
            self.config.ensemble_scale = Some(next_scale);
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
        self.log_scale_change(trigger, steps, previous_scale, next_scale);
        self.update_placement_after_scale_change();
        self.refresh_window_layout(ctx, previous_layout);
        if let Some(outer_position) = mouse_wheel_zoom_outer_position(
            trigger,
            previous_layout,
            self.window_layout,
            previous_viewport_info,
            self.placement_state.selected_anchor_kind,
            screen_safe_rect(ctx, &self.native_window_handle),
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
        }
        self.log_scale_layout_change(
            trigger,
            steps,
            previous_scale,
            next_scale,
            previous_layout,
            previous_viewport_info,
        );
        ctx.request_repaint();
        Ok(())
    }

    pub(super) fn pending_scale_persist_remaining(&self, now: Instant) -> Option<Duration> {
        pending_scale_persist_remaining_at(
            self.pending_persisted_scale,
            self.last_scale_change_at,
            now,
        )
    }

    pub(super) fn persist_pending_scale_if_due(&mut self, now: Instant) -> Result<()> {
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

    pub(super) fn persist_pending_scale(&mut self, scale: f32) -> Result<()> {
        if self.config.ensemble_mode.is_ensemble() {
            persist_ensemble_scale(&self.config_path, &self.config, scale)?;
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
}

pub(crate) fn mouse_wheel_zoom_outer_position(
    trigger: ScaleChangeTrigger,
    previous_layout: MascotWindowLayout,
    next_layout: MascotWindowLayout,
    previous_viewport_info: Option<ViewportInfo>,
    selected_anchor_kind: PlacementAnchorKind,
    screen_safe_rect: ScreenRectPx,
) -> Option<egui::Pos2> {
    let ScaleChangeTrigger::MouseWheel { .. } = trigger else {
        return None;
    };
    let viewport_info = previous_viewport_info?;
    clamp_zoomed_inner_origin_to_right_edge(
        viewport_info.inner_origin,
        previous_layout,
        next_layout,
        selected_anchor_kind,
        screen_safe_rect,
    )
    .map(|inner_origin| inner_origin - viewport_info.inner_to_outer_offset)
}

pub(crate) fn pending_scale_persist_remaining_at(
    pending_persisted_scale: Option<f32>,
    last_scale_change_at: Option<Instant>,
    now: Instant,
) -> Option<Duration> {
    match (pending_persisted_scale, last_scale_change_at) {
        (Some(_), Some(changed_at)) => {
            let elapsed = now.saturating_duration_since(changed_at);
            Some(SCALE_PERSIST_DEBOUNCE.saturating_sub(elapsed))
        }
        (None, None) => None,
        _ => {
            debug_assert!(
                matches!(
                    (pending_persisted_scale, last_scale_change_at),
                    (Some(_), Some(_)) | (None, None)
                ),
                "pending scale debounce state should be set and cleared together"
            );
            None
        }
    }
}
