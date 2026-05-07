use std::path::Path;

use eframe::egui::{self, Pos2};
use mascot_render_control::{log_server_error, log_server_info};
use mascot_render_protocol::{
    PlacementAnchorKind, PlacementAnchorPlan, PlacementAnchorPositions, PlacementMode,
    PlacementTargetScope, ScreenRectPx, ServerPlacementStatus, VisualSizePx,
};
use mascot_render_server::placement::{
    anchor_offset_array, anchor_position, anchor_positions_from_inner_origin, build_anchor_plan,
    save_placement_state, shared_height_scale, visual_size_px, PlacementPlanTargetInput,
};
use mascot_render_server::{MascotWindowLayout, PsdPlacementKey};

use super::character::ResolvedCharacterSkin;
use super::{CachedSkin, MascotApp, NativeWindowHandle};
use crate::app_support::size_vec;
use crate::mascot_scale::MIN_MASCOT_SCALE;

pub(super) struct PreparedPlacementChange {
    pub(super) scale: f32,
    pub(super) base_size: egui::Vec2,
    pub(super) anchor_position: Pos2,
    pub(super) anchor_kind: PlacementAnchorKind,
    pub(super) anchor_plan: PlacementAnchorPlan,
}

impl MascotApp {
    pub(super) fn initialize_current_placement_state(
        &mut self,
        restored_bottom_center: Option<Pos2>,
    ) {
        let anchor_positions = restored_bottom_center
            .map(|bottom_center| {
                let inner_origin = bottom_center - self.window_layout.bottom_center_anchor_offset();
                anchor_positions_from_inner_origin(inner_origin, self.window_layout)
            })
            .unwrap_or_else(|| anchor_positions_from_inner_origin(Pos2::ZERO, self.window_layout));
        let changed = self.placement_state.ensure_psd_state(
            self.current_psd_key(),
            self.scale,
            anchor_positions,
            self.current_visual_size_px(),
        ) || self.ensure_shared_state(anchor_positions);
        if changed {
            self.persist_placement_state("initialize_current_placement_state");
        }
    }

    pub(super) fn placement_status_for_snapshot(
        &mut self,
        ctx: &egui::Context,
    ) -> ServerPlacementStatus {
        let anchor_plan = self.anchor_plan_for_targets(
            ctx,
            self.current_target_scope(),
            self.current_plan_targets(ctx),
        );
        self.placement_state.status(anchor_plan)
    }

    pub(super) fn observe_current_placement_anchor_positions(
        &mut self,
        anchor_positions: PlacementAnchorPositions,
    ) {
        let changed = match self.placement_state.mode {
            PlacementMode::PerPsd => self
                .placement_state
                .update_psd_anchor_positions(self.current_psd_key(), anchor_positions),
            PlacementMode::SharedVisualSize => self
                .placement_state
                .update_shared_anchor_positions(anchor_positions),
        };
        if changed {
            self.persist_placement_state("observe_anchor_positions");
        }
    }

    pub(super) fn save_current_placement_anchor_positions(&mut self, ctx: &egui::Context) {
        let anchor_positions = self.current_anchor_positions(ctx);
        let changed = self
            .placement_state
            .update_psd_anchor_positions(self.current_psd_key(), anchor_positions);
        if changed {
            self.persist_placement_state("save_current_psd_anchor_positions");
        }
    }

    pub(super) fn save_current_placement_scale(&mut self) {
        let changed = self.placement_state.update_psd_scale(
            self.current_psd_key(),
            self.scale,
            self.current_visual_size_px(),
        );
        if changed {
            self.persist_placement_state("save_current_psd_scale");
        }
    }

    pub(super) fn update_placement_after_scale_change(&mut self) {
        let changed = match self.placement_state.mode {
            PlacementMode::PerPsd => self.placement_state.update_psd_scale(
                self.current_psd_key(),
                self.scale,
                self.current_visual_size_px(),
            ),
            PlacementMode::SharedVisualSize => self
                .placement_state
                .update_shared_visual_size(self.current_visual_size_px()),
        };
        if changed {
            self.persist_placement_state("update_after_scale_change");
        }
    }

    pub(super) fn set_placement_mode(&mut self, ctx: &egui::Context, mode: PlacementMode) {
        if self.placement_state.mode == mode {
            return;
        }
        let anchors = self.current_anchor_positions(ctx);
        self.placement_state.mode = mode;
        let changed = match mode {
            PlacementMode::PerPsd => self.placement_state.ensure_psd_state(
                self.current_psd_key(),
                self.scale,
                anchors,
                self.current_visual_size_px(),
            ),
            PlacementMode::SharedVisualSize => self.ensure_shared_state(anchors),
        };
        if changed || self.placement_state.mode == mode {
            self.persist_placement_state("set_placement_mode");
        }
    }

    pub(super) fn prepare_placement_for_character_change(
        &mut self,
        ctx: &egui::Context,
        resolved: &ResolvedCharacterSkin,
        next_skin: &CachedSkin,
    ) -> PreparedPlacementChange {
        self.prepare_placement_change(
            ctx,
            &PsdPlacementKey::new(resolved.zip_path.clone(), resolved.psd_path_in_zip.clone()),
            next_skin,
            None,
            "change_character",
        )
    }

    pub(super) fn prepare_placement_for_preview_target(
        &mut self,
        ctx: &egui::Context,
        zip_path: &Path,
        psd_path_in_zip: &Path,
        next_skin: &CachedSkin,
        requested_scale: Option<f32>,
    ) -> PreparedPlacementChange {
        self.prepare_placement_change(
            ctx,
            &PsdPlacementKey::new(zip_path.to_path_buf(), psd_path_in_zip.to_path_buf()),
            next_skin,
            requested_scale,
            "preview_target",
        )
    }

    fn prepare_placement_change(
        &mut self,
        ctx: &egui::Context,
        next_key: &PsdPlacementKey,
        next_skin: &CachedSkin,
        requested_scale: Option<f32>,
        action: &str,
    ) -> PreparedPlacementChange {
        let current_anchors = self.current_anchor_positions(ctx);
        let current_key = self.current_psd_key();
        let current_visual_size = self.current_visual_size_px();
        let mut changed = self.placement_state.ensure_psd_state(
            current_key,
            self.scale,
            current_anchors,
            current_visual_size,
        );
        changed |= self.ensure_shared_state(current_anchors);

        let next_scale = requested_scale
            .filter(|scale| scale.is_finite() && *scale >= MIN_MASCOT_SCALE)
            .unwrap_or_else(|| self.next_scale_for_skin(next_key, next_skin));
        let next_base_size = size_vec(
            next_skin.image_size[0],
            next_skin.image_size[1],
            Some(next_scale),
        );
        let next_visual_size = visual_size_for_skin(next_skin, next_scale);
        let target_anchors = self.next_anchor_positions(next_key, current_anchors);
        if self.placement_state.mode == PlacementMode::PerPsd {
            changed |= self.placement_state.ensure_psd_state(
                next_key.clone(),
                next_scale,
                target_anchors,
                next_visual_size,
            );
        }

        let next_layout = MascotWindowLayout::new(
            next_base_size,
            next_skin.image_size,
            next_skin.content_bounds,
            self.config.bounce,
            self.config.squash_bounce,
            self.config.always_idle_sink,
        );
        let anchor_plan = self.anchor_plan_for_targets(
            ctx,
            PlacementTargetScope::CandidatePsdSet,
            vec![
                self.current_plan_target(ctx),
                plan_target_input(
                    &next_key.zip_path,
                    &next_key.psd_path_in_zip,
                    next_scale,
                    next_visual_size,
                    target_anchors,
                    next_layout,
                ),
            ],
        );
        let anchor_kind = anchor_plan.selected_anchor_kind;
        let anchor_position = anchor_position(target_anchors, anchor_kind);
        if changed {
            self.persist_placement_state(match action {
                "preview_target" => "prepare_preview_target_change",
                _ => "prepare_character_change",
            });
        }
        log_server_info(format!(
            "event=placement_anchor_plan action={action} placement_mode={:?} anchor_policy={:?} selected_anchor_kind={:?} target_scope={:?} target_count={} max_right_overflow_px={} selected_zip={} selected_psd={}",
            anchor_plan.placement_mode,
            anchor_plan.anchor_policy,
            anchor_plan.selected_anchor_kind,
            anchor_plan.target_scope,
            anchor_plan.target_count,
            anchor_plan.max_right_overflow_px,
            next_key.zip_path.display(),
            next_key.psd_path_in_zip.display(),
        ));
        PreparedPlacementChange {
            scale: next_scale,
            base_size: next_base_size,
            anchor_position,
            anchor_kind,
            anchor_plan,
        }
    }

    pub(super) fn current_anchor_positions(&self, ctx: &egui::Context) -> PlacementAnchorPositions {
        if let Some(viewport_info) =
            mascot_render_server::window_history::current_viewport_info(ctx)
        {
            return anchor_positions_from_inner_origin(
                viewport_info.inner_origin,
                self.window_layout,
            );
        }
        self.saved_anchor_positions()
            .or_else(|| {
                self.pending_restored_anchor_position.map(|bottom_center| {
                    let inner_origin =
                        bottom_center - self.window_layout.bottom_center_anchor_offset();
                    anchor_positions_from_inner_origin(inner_origin, self.window_layout)
                })
            })
            .unwrap_or_else(|| anchor_positions_from_inner_origin(Pos2::ZERO, self.window_layout))
    }

    pub(super) fn current_psd_key(&self) -> PsdPlacementKey {
        PsdPlacementKey::new(
            self.config.zip_path.clone(),
            self.config.psd_path_in_zip.clone(),
        )
    }

    pub(super) fn current_visual_size_px(&self) -> VisualSizePx {
        visual_size_px(self.window_content_bounds(), self.scale).unwrap_or_else(|| VisualSizePx {
            width: (self.open_skin.image_size[0] as f32 * self.scale).max(1.0),
            height: (self.open_skin.image_size[1] as f32 * self.scale).max(1.0),
        })
    }

    pub(super) fn restore_anchor_position_for_kind(
        &self,
        ctx: &egui::Context,
        anchor_position: Pos2,
        anchor_kind: PlacementAnchorKind,
    ) {
        let anchor_offset = self.window_layout.anchor_offset_for_kind(anchor_kind);
        let outer_position = mascot_render_server::window_history::current_viewport_info(ctx)
            .map_or(anchor_position - anchor_offset, |viewport_info| {
                mascot_render_server::window_history::outer_position_for_anchor(
                    anchor_position,
                    anchor_offset,
                    viewport_info.inner_to_outer_offset,
                )
            });
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer_position));
    }

    fn anchor_plan_for_targets(
        &mut self,
        ctx: &egui::Context,
        target_scope: PlacementTargetScope,
        targets: Vec<PlacementPlanTargetInput>,
    ) -> PlacementAnchorPlan {
        let plan = build_anchor_plan(
            self.placement_state.mode,
            target_scope,
            screen_safe_rect(ctx, &self.native_window_handle),
            targets,
        );
        self.placement_state.selected_anchor_kind = plan.selected_anchor_kind;
        plan
    }

    fn current_plan_target(&self, ctx: &egui::Context) -> PlacementPlanTargetInput {
        let key = self.current_psd_key();
        plan_target_input(
            &key.zip_path,
            &key.psd_path_in_zip,
            self.scale,
            self.current_visual_size_px(),
            self.current_anchor_positions(ctx),
            self.window_layout,
        )
    }

    fn current_plan_targets(&self, ctx: &egui::Context) -> Vec<PlacementPlanTargetInput> {
        if self.config.favorite_ensemble_enabled {
            if let Some(favorite_ensemble) = &self.favorite_ensemble {
                let anchors = self.current_anchor_positions(ctx);
                let bottom_center = anchor_position(anchors, PlacementAnchorKind::BottomCenter);
                let inner_origin = bottom_center - self.window_layout.bottom_center_anchor_offset();
                let targets = favorite_ensemble.placement_plan_targets(
                    inner_origin,
                    self.window_layout.canvas_origin_offset(self.base_size),
                    self.scale,
                );
                if !targets.is_empty() {
                    return targets;
                }
            }
        }
        vec![self.current_plan_target(ctx)]
    }

    fn current_target_scope(&self) -> PlacementTargetScope {
        if self.config.favorite_ensemble_enabled {
            PlacementTargetScope::FavoriteEnsemble
        } else {
            PlacementTargetScope::CurrentPsd
        }
    }

    fn next_scale_for_skin(&self, key: &PsdPlacementKey, next_skin: &CachedSkin) -> f32 {
        match self.placement_state.mode {
            PlacementMode::PerPsd => self
                .placement_state
                .psd_state(key)
                .and_then(|state| state.scale)
                .unwrap_or(self.scale),
            PlacementMode::SharedVisualSize => self
                .placement_state
                .shared_visual_size_px
                .and_then(|size| {
                    shared_height_scale(
                        size,
                        next_skin.content_bounds,
                        next_skin.image_size,
                        MIN_MASCOT_SCALE,
                    )
                })
                .unwrap_or(self.scale),
        }
    }

    fn next_anchor_positions(
        &self,
        key: &PsdPlacementKey,
        current_anchors: PlacementAnchorPositions,
    ) -> PlacementAnchorPositions {
        match self.placement_state.mode {
            PlacementMode::PerPsd => self
                .placement_state
                .psd_state(key)
                .and_then(|state| state.anchor_positions)
                .unwrap_or(current_anchors),
            PlacementMode::SharedVisualSize => self
                .placement_state
                .shared_anchor_positions
                .unwrap_or(current_anchors),
        }
    }

    fn saved_anchor_positions(&self) -> Option<PlacementAnchorPositions> {
        match self.placement_state.mode {
            PlacementMode::PerPsd => self
                .placement_state
                .psd_state(&self.current_psd_key())
                .and_then(|state| state.anchor_positions),
            PlacementMode::SharedVisualSize => self.placement_state.shared_anchor_positions,
        }
    }

    fn ensure_shared_state(&mut self, anchor_positions: PlacementAnchorPositions) -> bool {
        if self.placement_state.mode != PlacementMode::SharedVisualSize {
            return false;
        }
        self.placement_state
            .update_shared_anchor_positions(anchor_positions)
            | self
                .placement_state
                .update_shared_visual_size(self.current_visual_size_px())
    }

    fn persist_placement_state(&self, stage: &str) {
        match save_placement_state(&self.placement_state_path, &self.placement_state) {
            Ok(()) => log_server_info(format!(
                "event=placement_state stage={stage} result=saved path={}",
                self.placement_state_path.display()
            )),
            Err(error) => log_server_error(format!(
                "event=placement_state stage={stage} result=failed path={} error={error:#}",
                self.placement_state_path.display()
            )),
        }
    }
}

fn visual_size_for_skin(skin: &CachedSkin, scale: f32) -> VisualSizePx {
    visual_size_px(skin.content_bounds, scale).unwrap_or_else(|| VisualSizePx {
        width: (skin.image_size[0] as f32 * scale).max(1.0),
        height: (skin.image_size[1] as f32 * scale).max(1.0),
    })
}

fn plan_target_input(
    zip_path: &Path,
    psd_path_in_zip: &Path,
    scale: f32,
    visible_size_px: VisualSizePx,
    anchor_positions: PlacementAnchorPositions,
    layout: MascotWindowLayout,
) -> PlacementPlanTargetInput {
    PlacementPlanTargetInput {
        zip_path: zip_path.to_path_buf(),
        psd_path_in_zip: psd_path_in_zip.to_path_buf(),
        scale,
        visible_size_px,
        bottom_center_anchor_position: anchor_positions.bottom_center,
        bottom_right_anchor_position: anchor_positions.bottom_right,
        bottom_center_anchor_offset: anchor_offset_array(layout.bottom_center_anchor_offset()),
        bottom_right_anchor_offset: anchor_offset_array(layout.bottom_right_anchor_offset()),
    }
}

pub(super) fn screen_safe_rect(
    ctx: &egui::Context,
    native_window_handle: &NativeWindowHandle,
) -> ScreenRectPx {
    if let Some(rect) = native_window_handle.monitor_screen_rect() {
        return rect;
    }
    ctx.input(|input| {
        let viewport = input.viewport();
        if let Some(monitor_size) = viewport.monitor_size {
            return ScreenRectPx {
                min_x: 0.0,
                min_y: 0.0,
                max_x: monitor_size.x,
                max_y: monitor_size.y,
            };
        }
        viewport
            .inner_rect
            .map_or_else(ScreenRectPx::default, |rect| ScreenRectPx {
                min_x: 0.0,
                min_y: 0.0,
                max_x: rect.max.x.max(rect.width()),
                max_y: rect.max.y.max(rect.height()),
            })
    })
}
