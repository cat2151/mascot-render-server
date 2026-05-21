use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, Vec2};
use mascot_render_core::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionState, SquashBounceAnimationConfig,
};
use mascot_render_protocol::VisualSizePx;
use mascot_render_server::{AlphaBounds, PlacementPlanTargetInput};

use super::mouth_flap_state::{active_skin_state, ActiveSkinState};
use crate::app_support::{cached_skin_from_image, CachedSkin};
use crate::ensemble::{Ensemble, EnsembleMember};
use crate::eye_blink::EyeBlinkLoop;
use crate::eye_blink_timing::always_idle_sink_for_blink_median;

pub(super) struct EnsembleMemberScene {
    pub(super) character_name: Option<String>,
    pub(super) zip_path: PathBuf,
    pub(super) psd_path_in_zip: PathBuf,
    pub(super) origin: Pos2,
    pub(super) base_size: Vec2,
    pub(super) open_skin: CachedSkin,
    pub(super) closed_skin: Option<CachedSkin>,
    mouth_open_skin: Option<CachedSkin>,
    mouth_closed_skin: Option<CachedSkin>,
    pub(super) motion: MotionState,
    pub(super) eye_blink: EyeBlinkLoop,
    pub(super) phase_offset_ratio: f32,
}

pub(crate) struct EnsembleScene {
    pub(super) members: Vec<EnsembleMemberScene>,
    canvas_size: Vec2,
}

impl EnsembleScene {
    pub(super) fn from_loaded(
        ctx: &egui::Context,
        ensemble: Ensemble,
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

    pub(super) fn placement_plan_targets(
        &self,
        inner_origin: Pos2,
        canvas_origin: Vec2,
        scale: f32,
    ) -> Vec<PlacementPlanTargetInput> {
        self.members
            .iter()
            .map(|member| member.placement_plan_target(inner_origin, canvas_origin, scale))
            .collect()
    }

    pub(super) fn set_always_idle_sink_enabled(&mut self, enabled: bool, now: Instant) {
        for member in &mut self.members {
            member.motion.set_always_idle_sink_enabled(enabled, now);
        }
    }

    pub(super) fn trigger_mouth_flap_for_character(
        &mut self,
        character_name: &str,
        now: Instant,
        duration: Duration,
        fps: u16,
    ) -> bool {
        let Some(member) = self
            .members
            .iter_mut()
            .find(|member| member.character_name() == Some(character_name))
        else {
            return false;
        };
        member.motion.trigger_mouth_flap(now, duration, fps);
        true
    }

    pub(super) fn trigger_bounce_for_character(
        &mut self,
        character_name: &str,
        now: Instant,
    ) -> bool {
        let Some(member) = self
            .members
            .iter_mut()
            .find(|member| member.character_name() == Some(character_name))
        else {
            return false;
        };
        member.motion.trigger_bounce(now);
        true
    }

    pub(super) fn mouth_flap_is_open(&mut self, now: Instant) -> Option<bool> {
        let mut any_closed = false;
        for member in &mut self.members {
            match member.mouth_flap_is_open(now) {
                Some(true) => return Some(true),
                Some(false) => any_closed = true,
                None => {}
            }
        }
        any_closed.then_some(false)
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
                let eye_blink_repaint_after = member
                    .closed_skin
                    .as_ref()
                    .map(|_| member.eye_blink.deadline_after(now));
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

#[cfg(test)]
impl EnsembleScene {
    pub(crate) fn from_loaded_for_test(
        ctx: &egui::Context,
        ensemble: Ensemble,
        always_idle_sink_enabled: bool,
        now: Instant,
    ) -> Self {
        Self::from_loaded(ctx, ensemble, always_idle_sink_enabled, now)
    }

    pub(crate) fn trigger_bounce_for_character_for_test(
        &mut self,
        character_name: &str,
        now: Instant,
    ) -> bool {
        self.trigger_bounce_for_character(character_name, now)
    }

    pub(crate) fn member_motion_is_active_for_test(&self, index: usize) -> bool {
        self.members[index].motion.is_active()
    }
}

fn member_scene_from_loaded(
    ctx: &egui::Context,
    member: EnsembleMember,
    always_idle_sink_enabled: bool,
    now: Instant,
    member_index: usize,
    member_count: usize,
) -> EnsembleMemberScene {
    let phase_offset_ratio = member_phase_offset_ratio(member_index, member_count);
    let mut motion = MotionState::new_with_idle_phase_offset(phase_offset_ratio);
    motion.set_always_idle_sink_enabled(always_idle_sink_enabled, now);
    EnsembleMemberScene {
        character_name: member.character_name,
        zip_path: member.zip_path,
        psd_path_in_zip: member.psd_path_in_zip,
        origin: Pos2::new(member.canvas_position[0], member.canvas_position[1]),
        base_size: Vec2::new(member.base_size[0], member.base_size[1]),
        open_skin: cached_skin_from_image(ctx, &member.image),
        closed_skin: member
            .closed_image
            .as_ref()
            .map(|image| cached_skin_from_image(ctx, image)),
        mouth_open_skin: member
            .mouth_open_image
            .as_ref()
            .map(|image| cached_skin_from_image(ctx, image)),
        mouth_closed_skin: member
            .mouth_closed_image
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

impl EnsembleMemberScene {
    pub(super) fn character_name(&self) -> Option<&str> {
        self.character_name.as_deref()
    }

    pub(super) fn active_skin(&mut self, blink_closed: bool, now: Instant) -> &CachedSkin {
        match active_skin_state(
            self.has_mouth_flap_skin(),
            &mut self.motion,
            blink_closed,
            now,
        ) {
            ActiveSkinState::MouthOpen => self.mouth_open_skin.as_ref().unwrap_or(&self.open_skin),
            ActiveSkinState::MouthClosed => {
                self.mouth_closed_skin.as_ref().unwrap_or(&self.open_skin)
            }
            ActiveSkinState::BlinkClosed => self.closed_skin.as_ref().unwrap_or(&self.open_skin),
            ActiveSkinState::Open => &self.open_skin,
        }
    }

    fn has_mouth_flap_skin(&self) -> bool {
        self.mouth_open_skin.is_some() || self.mouth_closed_skin.is_some()
    }

    fn mouth_flap_is_open(&mut self, now: Instant) -> Option<bool> {
        self.has_mouth_flap_skin()
            .then(|| self.motion.mouth_flap_is_open(now))
            .flatten()
    }

    fn placement_plan_target(
        &self,
        inner_origin: Pos2,
        canvas_origin: Vec2,
        scale: f32,
    ) -> PlacementPlanTargetInput {
        let scale = scale.max(0.01);
        let image_size = self.open_skin.image_size;
        let base_size = self.base_size * scale;
        let image_origin = canvas_origin + self.origin.to_vec2() * scale;
        let [min_x, min_y, max_x, max_y] = scaled_content_bounds(
            image_size,
            self.open_skin.content_bounds,
            image_origin,
            base_size,
        );
        let bottom_center_offset = Vec2::new((min_x + max_x) * 0.5, max_y);
        let bottom_right_offset = Vec2::new(max_x, max_y);
        let bottom_center = inner_origin + bottom_center_offset;
        let bottom_right = inner_origin + bottom_right_offset;
        PlacementPlanTargetInput {
            zip_path: self.zip_path.clone(),
            psd_path_in_zip: self.psd_path_in_zip.clone(),
            scale: member_scale(image_size, base_size),
            visible_size_px: VisualSizePx {
                width: (max_x - min_x).max(1.0),
                height: (max_y - min_y).max(1.0),
            },
            bottom_center_anchor_position: [bottom_center.x, bottom_center.y],
            bottom_right_anchor_position: [bottom_right.x, bottom_right.y],
            bottom_center_anchor_offset: [bottom_center_offset.x, bottom_center_offset.y],
            bottom_right_anchor_offset: [bottom_right_offset.x, bottom_right_offset.y],
        }
    }
}

fn scaled_content_bounds(
    image_size: [u32; 2],
    bounds: AlphaBounds,
    image_origin: Vec2,
    base_size: Vec2,
) -> [f32; 4] {
    let width = image_size[0].max(1) as f32;
    let height = image_size[1].max(1) as f32;
    [
        image_origin.x + base_size.x * (bounds.min_x as f32 / width),
        image_origin.y + base_size.y * (bounds.min_y as f32 / height),
        image_origin.x + base_size.x * (bounds.max_x as f32 / width),
        image_origin.y + base_size.y * (bounds.max_y as f32 / height),
    ]
}

fn member_scale(image_size: [u32; 2], base_size: Vec2) -> f32 {
    let width = image_size[0].max(1) as f32;
    (base_size.x / width).max(0.01)
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

#[cfg(test)]
mod tests {
    use mascot_render_core::MascotImageData;

    use super::*;

    #[test]
    fn ensemble_plan_targets_include_each_member_source_key() {
        let ctx = egui::Context::default();
        let scene = EnsembleScene::from_loaded(
            &ctx,
            Ensemble {
                canvas_size: [30.0, 20.0],
                members: vec![
                    member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]),
                    member("b.zip", "b.psd", [10.0, 0.0], [20.0, 20.0]),
                ],
            },
            false,
            Instant::now(),
        );

        let targets = scene.placement_plan_targets(Pos2::new(100.0, 200.0), Vec2::ZERO, 1.0);

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].zip_path, PathBuf::from("a.zip"));
        assert_eq!(targets[0].psd_path_in_zip, PathBuf::from("a.psd"));
        assert_eq!(targets[1].zip_path, PathBuf::from("b.zip"));
        assert_eq!(targets[1].psd_path_in_zip, PathBuf::from("b.psd"));
        assert_eq!(targets[1].bottom_right_anchor_offset, [30.0, 20.0]);
    }

    #[test]
    fn ensemble_triggers_mouth_flap_for_named_member_only() {
        let ctx = egui::Context::default();
        let mut first = member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]);
        first.character_name = Some("ずんだもん".to_string());
        let mut second = member("b.zip", "b.psd", [10.0, 0.0], [20.0, 20.0]);
        second.character_name = Some("四国めたん".to_string());
        let mut scene = EnsembleScene::from_loaded(
            &ctx,
            Ensemble {
                canvas_size: [30.0, 20.0],
                members: vec![first, second],
            },
            false,
            Instant::now(),
        );

        assert!(scene.trigger_mouth_flap_for_character(
            "四国めたん",
            Instant::now(),
            Duration::from_secs(1),
            4,
        ));

        assert!(!scene.members[0].motion.is_active());
        assert!(scene.members[1].motion.is_active());
    }

    #[test]
    fn ensemble_member_active_skin_prefers_mouth_flap_over_blink() {
        let ctx = egui::Context::default();
        let mut member = member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]);
        member.closed_image = Some(image("blink-closed", [10.0, 20.0]));
        member.mouth_open_image = Some(image("mouth-open", [10.0, 20.0]));
        member.mouth_closed_image = Some(image("mouth-closed", [10.0, 20.0]));
        let now = Instant::now();
        let mut scene = EnsembleScene::from_loaded(
            &ctx,
            Ensemble {
                canvas_size: [10.0, 20.0],
                members: vec![member],
            },
            false,
            now,
        );

        scene.members[0]
            .motion
            .trigger_mouth_flap(now, Duration::from_secs(1), 4);

        assert_eq!(
            scene.members[0].active_skin(true, now).path.clone(),
            PathBuf::from("mouth-open.png")
        );
        assert_eq!(
            scene.members[0]
                .active_skin(true, now + Duration::from_millis(250))
                .path
                .clone(),
            PathBuf::from("mouth-closed.png")
        );
        assert_eq!(
            scene.members[0]
                .active_skin(true, now + Duration::from_secs(1))
                .path
                .clone(),
            PathBuf::from("blink-closed.png")
        );
    }

    fn member(
        zip_path: &str,
        psd_path_in_zip: &str,
        canvas_position: [f32; 2],
        base_size: [f32; 2],
    ) -> EnsembleMember {
        EnsembleMember {
            character_name: None,
            zip_path: PathBuf::from(zip_path),
            psd_path_in_zip: PathBuf::from(psd_path_in_zip),
            image: image(zip_path, base_size),
            closed_image: None,
            mouth_open_image: None,
            mouth_closed_image: None,
            base_size,
            canvas_position,
        }
    }

    fn image(path: &str, size: [f32; 2]) -> MascotImageData {
        let width = size[0].ceil().max(1.0) as u32;
        let height = size[1].ceil().max(1.0) as u32;
        MascotImageData {
            path: PathBuf::from(path).with_extension("png"),
            width,
            height,
            rgba: vec![255; width as usize * height as usize * 4],
        }
    }
}
