use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, Vec2};
use mascot_render_core::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionState, SquashBounceAnimationConfig,
};
use mascot_render_server::AlphaBounds;

use crate::app_support::{cached_skin_from_image, CachedSkin};
use crate::eye_blink::EyeBlinkLoop;
use crate::eye_blink_timing::always_idle_sink_for_blink_median;
use crate::favorite_ensemble::{FavoriteEnsemble, FavoriteEnsembleMember};

pub(super) struct FavoriteEnsembleMemberScene {
    pub(super) origin: Pos2,
    pub(super) base_size: Vec2,
    pub(super) open_skin: CachedSkin,
    pub(super) closed_skin: Option<CachedSkin>,
    pub(super) motion: MotionState,
    pub(super) eye_blink: EyeBlinkLoop,
    pub(super) phase_offset_ratio: f32,
}

pub(super) struct FavoriteEnsembleScene {
    pub(super) members: Vec<FavoriteEnsembleMemberScene>,
    canvas_size: Vec2,
}

impl FavoriteEnsembleScene {
    pub(super) fn from_loaded(
        ctx: &egui::Context,
        ensemble: FavoriteEnsemble,
        always_idle_sink_enabled: bool,
        now: Instant,
    ) -> Self {
        let member_count = ensemble.members.len();
        let mut members = ensemble
            .members
            .into_iter()
            .enumerate()
            .map(|(member_index, member)| {
                member_scene_from_loaded(
                    ctx,
                    member,
                    always_idle_sink_enabled,
                    now,
                    member_index,
                    member_count,
                )
            })
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

    pub(super) fn set_always_idle_sink_enabled(&mut self, enabled: bool, now: Instant) {
        for member in &mut self.members {
            member.motion.set_always_idle_sink_enabled(enabled, now);
        }
    }

    pub(super) fn repaint_after(
        &mut self,
        now: Instant,
        bounce: BounceAnimationConfig,
        squash_bounce: SquashBounceAnimationConfig,
        always_idle_sink: IdleSinkAnimationConfig,
    ) -> Option<Duration> {
        self.members
            .iter_mut()
            .filter_map(|member| {
                let motion_repaint_after = member.motion.repaint_after(
                    now,
                    bounce,
                    squash_bounce,
                    always_idle_sink_for_blink_median(
                        always_idle_sink,
                        member.eye_blink.current_median_ms(),
                    ),
                );
                let eye_blink_repaint_after = member.closed_skin.as_ref().map(|_| {
                    member
                        .eye_blink
                        .repaint_after(now, Duration::from_millis(250))
                });
                match (motion_repaint_after, eye_blink_repaint_after) {
                    (Some(motion_repaint_after), Some(eye_blink_repaint_after)) => {
                        Some(motion_repaint_after.min(eye_blink_repaint_after))
                    }
                    (Some(motion_repaint_after), None) => Some(motion_repaint_after),
                    (None, Some(eye_blink_repaint_after)) => Some(eye_blink_repaint_after),
                    (None, None) => None,
                }
            })
            .min()
    }
}

fn member_scene_from_loaded(
    ctx: &egui::Context,
    member: FavoriteEnsembleMember,
    always_idle_sink_enabled: bool,
    now: Instant,
    member_index: usize,
    member_count: usize,
) -> FavoriteEnsembleMemberScene {
    let phase_offset_ratio = member_phase_offset_ratio(member_index, member_count);
    let mut motion = MotionState::new_with_idle_phase_offset(phase_offset_ratio);
    motion.set_always_idle_sink_enabled(always_idle_sink_enabled, now);
    FavoriteEnsembleMemberScene {
        origin: Pos2::new(member.canvas_position[0], member.canvas_position[1]),
        base_size: Vec2::new(member.base_size[0], member.base_size[1]),
        open_skin: cached_skin_from_image(ctx, &member.image),
        closed_skin: member
            .closed_image
            .as_ref()
            .map(|image| cached_skin_from_image(ctx, image)),
        motion,
        eye_blink: EyeBlinkLoop::new_with_seed_and_elapsed(
            now,
            member_eye_blink_seed(member_index, member_count),
            member_eye_blink_elapsed(member_index, member_count),
        ),
        phase_offset_ratio,
    }
}

pub(crate) fn member_phase_offset_ratio(member_index: usize, member_count: usize) -> f32 {
    if member_count <= 1 {
        return 0.0;
    }
    assert!(
        member_index < member_count,
        "member_index must be less than member_count: member_index={member_index}, member_count={member_count}"
    );
    member_index as f32 / member_count as f32
}

pub(crate) fn member_eye_blink_elapsed(member_index: usize, member_count: usize) -> Duration {
    Duration::from_secs_f32(member_phase_offset_ratio(member_index, member_count))
}

pub(crate) fn member_eye_blink_seed(member_index: usize, member_count: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    member_count.hash(&mut hasher);
    member_index.hash(&mut hasher);
    hasher.finish()
}
