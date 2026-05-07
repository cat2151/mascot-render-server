use eframe::egui::{self, Pos2};
use mascot_render_server::window_history::{current_viewport_info, outer_position_for_anchor};
use mascot_render_server::AlphaBounds;

use super::logging::RefreshWindowLayoutDiagnostics;
use super::{ensemble_window_layout, MascotApp};
use mascot_render_server::{anchored_inner_origin_for_kind, MascotWindowLayout};

pub(super) fn apply_pending_restored_anchor_position(app: &mut MascotApp, ctx: &egui::Context) {
    let Some(anchor_position) = app.pending_restored_anchor_position else {
        return;
    };
    if current_viewport_info(ctx).is_none() {
        return;
    }
    restore_anchor_position(app, ctx, anchor_position);
    app.pending_restored_anchor_position = None;
}

pub(super) fn restore_anchor_position(
    app: &mut MascotApp,
    ctx: &egui::Context,
    anchor_position: Pos2,
) {
    let outer_position = current_viewport_info(ctx)
        .map(|viewport_info| {
            outer_position_for_anchor(
                anchor_position,
                app.window_layout.anchor_offset(),
                viewport_info.inner_to_outer_offset,
            )
        })
        // Before viewport info is available we can only place the window using the anchor
        // offset. `apply_pending_restored_anchor_position()` re-applies the restore on a later
        // frame once the measured frame offset becomes available.
        .unwrap_or(anchor_position - app.window_layout.anchor_offset());
    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
}

impl MascotApp {
    pub(super) fn refresh_window_layout(
        &mut self,
        ctx: &egui::Context,
        previous_layout: MascotWindowLayout,
    ) -> RefreshWindowLayoutDiagnostics {
        let viewport_info = current_viewport_info(ctx);
        let selected_anchor_kind = self.placement_state.selected_anchor_kind;
        self.window_layout = if let Some(favorite_ensemble) = &self.favorite_ensemble {
            self.base_size = favorite_ensemble.scaled_canvas_size(self.scale);
            ensemble_window_layout(self.base_size, favorite_ensemble.image_size(), &self.config)
        } else {
            let content_bounds = self.window_content_bounds();
            MascotWindowLayout::new(
                self.base_size,
                self.open_skin.image_size,
                content_bounds,
                self.config.bounce,
                self.config.squash_bounce,
                self.config.always_idle_sink,
            )
        };
        let preserved_anchor_position = viewport_info.map(|value| {
            value.inner_origin + previous_layout.anchor_offset_for_kind(selected_anchor_kind)
        });
        let next_inner_origin = viewport_info.map(|value| {
            anchored_inner_origin_for_kind(
                value.inner_origin,
                previous_layout,
                self.window_layout,
                selected_anchor_kind,
            )
        });
        let next_outer_position = next_inner_origin
            .zip(viewport_info.map(|value| value.inner_to_outer_offset))
            .map(|(inner_origin, inner_to_outer_offset)| inner_origin - inner_to_outer_offset);
        if let Some(outer_position) = next_outer_position {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
            self.window_layout.window_size(),
        ));
        RefreshWindowLayoutDiagnostics {
            selected_anchor_kind,
            previous_layout,
            next_layout: self.window_layout,
            viewport_info,
            preserved_anchor_position,
            next_inner_origin,
            next_outer_position,
        }
    }

    pub(super) fn window_content_bounds(&self) -> AlphaBounds {
        if let Some(favorite_ensemble) = &self.favorite_ensemble {
            return favorite_ensemble.content_bounds();
        }
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
        for (label, skin) in [
            ("mouth-open", self.mouth_open_skin.as_ref()),
            ("mouth-closed", self.mouth_closed_skin.as_ref()),
        ] {
            let Some(skin) = skin else {
                continue;
            };
            if skin.image_size == self.open_skin.image_size {
                bounds = bounds.union(skin.content_bounds);
            } else {
                eprintln!(
                    "{} skin size {:?} does not match open skin size {:?}; using open skin bounds for the window layout",
                    label,
                    skin.image_size,
                    self.open_skin.image_size
                );
            }
        }
        bounds
    }
}
