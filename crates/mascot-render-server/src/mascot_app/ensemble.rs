use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, Vec2};
use mascot_render_core::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionState, SquashBounceAnimationConfig,
};
use mascot_render_server::AlphaBounds;

use crate::app_support::{cached_skin_from_image, CachedSkin};
use crate::favorite_ensemble::{FavoriteEnsemble, FavoriteEnsembleMember};

pub(super) struct FavoriteEnsembleMemberScene {
    pub(super) origin: Pos2,
    pub(super) base_size: Vec2,
    pub(super) skin: CachedSkin,
    pub(super) motion: MotionState,
}

pub(super) struct FavoriteEnsembleScene {
    pub(super) members: Vec<FavoriteEnsembleMemberScene>,
    canvas_size: Vec2,
}

impl FavoriteEnsembleScene {
    pub(super) fn from_loaded(
        ctx: &egui::Context,
        ensemble: FavoriteEnsemble,
        always_bouncing: bool,
        now: Instant,
    ) -> Self {
        let mut members = ensemble
            .members
            .into_iter()
            .map(|member| member_scene_from_loaded(ctx, member, always_bouncing, now))
            .collect::<Vec<_>>();
        members.shrink_to_fit();

        Self {
            members,
            canvas_size: Vec2::new(ensemble.canvas_size[0], ensemble.canvas_size[1]),
        }
    }

    pub(super) fn scaled_canvas_size(&self, scale: f32) -> Vec2 {
        Vec2::new(
            (self.canvas_size.x * scale.max(0.01)).max(1.0),
            (self.canvas_size.y * scale.max(0.01)).max(1.0),
        )
    }

    pub(super) fn image_size(&self) -> [u32; 2] {
        [
            self.canvas_size.x.ceil().max(1.0) as u32,
            self.canvas_size.y.ceil().max(1.0) as u32,
        ]
    }

    pub(super) fn content_bounds(&self) -> AlphaBounds {
        AlphaBounds::full(self.image_size())
    }

    pub(super) fn set_always_bouncing(&mut self, enabled: bool, now: Instant) {
        for member in &mut self.members {
            member.motion.set_always_bouncing(enabled, now);
        }
    }

    pub(super) fn repaint_after(
        &self,
        now: Instant,
        bounce: BounceAnimationConfig,
        squash_bounce: SquashBounceAnimationConfig,
        always_idle_sink: IdleSinkAnimationConfig,
    ) -> Option<Duration> {
        self.members
            .iter()
            .filter_map(|member| {
                member
                    .motion
                    .repaint_after(now, bounce, squash_bounce, always_idle_sink)
            })
            .min()
    }
}

fn member_scene_from_loaded(
    ctx: &egui::Context,
    member: FavoriteEnsembleMember,
    always_bouncing: bool,
    now: Instant,
) -> FavoriteEnsembleMemberScene {
    let mut motion = MotionState::new();
    motion.set_always_bouncing(always_bouncing, now);
    FavoriteEnsembleMemberScene {
        origin: Pos2::new(member.canvas_position[0], member.canvas_position[1]),
        base_size: Vec2::new(member.base_size[0], member.base_size[1]),
        skin: cached_skin_from_image(ctx, &member.image),
        motion,
    }
}
