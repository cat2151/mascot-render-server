use std::time::{Duration, Instant};

use eframe::egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};

mod sampling;

const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);
pub const IDLE_SINK_LIFT_SCALE_X_RATIO: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BounceAlgorithm {
    #[default]
    DampedSine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SquashAlgorithm {
    #[default]
    SquashStretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IdleAlgorithm {
    #[default]
    IdleSink,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadHitbox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for HeadHitbox {
    fn default() -> Self {
        Self {
            x: 0.18,
            y: 0.02,
            width: 0.64,
            height: 0.42,
        }
    }
}

impl HeadHitbox {
    pub fn contains(self, image_rect: Rect, pointer_pos: Pos2) -> bool {
        if !image_rect.contains(pointer_pos) {
            return false;
        }

        let normalized_x = (pointer_pos.x - image_rect.min.x) / image_rect.width().max(1.0);
        let normalized_y = (pointer_pos.y - image_rect.min.y) / image_rect.height().max(1.0);
        normalized_x >= self.x
            && normalized_x <= self.x + self.width
            && normalized_y >= self.y
            && normalized_y <= self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BounceAnimationConfig {
    pub algorithm: BounceAlgorithm,
    pub duration_ms: u64,
    pub amplitude_px: f32,
    pub cycles: f32,
    pub damping: f32,
}

impl Default for BounceAnimationConfig {
    fn default() -> Self {
        Self {
            algorithm: BounceAlgorithm::DampedSine,
            duration_ms: 900,
            amplitude_px: 40.0,
            cycles: 1.75,
            damping: 2.6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SquashBounceAnimationConfig {
    pub algorithm: SquashAlgorithm,
    pub duration_ms: u64,
    pub amplitude_px: f32,
    pub cycles: f32,
    pub damping: f32,
    pub squash_amount: f32,
    pub stretch_amount: f32,
}

impl Default for SquashBounceAnimationConfig {
    fn default() -> Self {
        Self {
            algorithm: SquashAlgorithm::SquashStretch,
            duration_ms: 760,
            amplitude_px: 28.0,
            cycles: 1.5,
            damping: 2.2,
            squash_amount: 0.16,
            stretch_amount: 0.10,
        }
    }
}

impl SquashBounceAnimationConfig {
    pub fn default_for_always_bouncing() -> Self {
        Self {
            algorithm: SquashAlgorithm::SquashStretch,
            duration_ms: 1400,
            amplitude_px: 12.0,
            cycles: 1.2,
            damping: 2.8,
            squash_amount: 0.08,
            stretch_amount: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IdleSinkAnimationConfig {
    pub algorithm: IdleAlgorithm,
    pub duration_ms: u64,
    pub amplitude_px: f32,
    pub sink_amount: f32,
    pub lift_amount: f32,
}

impl Default for IdleSinkAnimationConfig {
    fn default() -> Self {
        Self {
            algorithm: IdleAlgorithm::IdleSink,
            duration_ms: 2200,
            amplitude_px: 6.0,
            sink_amount: 0.045,
            lift_amount: 0.03,
        }
    }
}

impl IdleSinkAnimationConfig {
    pub fn default_for_always_bouncing() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionTransform {
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl MotionTransform {
    pub fn identity() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationKind {
    Bounce,
    SquashBounce,
    IdleSink,
    Shake,
}

#[derive(Debug, Clone, Copy)]
struct ActiveAnimation {
    kind: AnimationKind,
    idle: bool,
    started_at: Instant,
    shake_amplitude_px: f32,
    shake_seed: u64,
    shake_duration: Duration,
    shake_frame_interval: Duration,
}

#[derive(Debug, Clone, Copy)]
struct MouthFlapAnimation {
    started_at: Instant,
    duration: Duration,
    frame_interval: Duration,
}

#[derive(Debug)]
pub struct MotionState {
    next_kind: Option<AnimationKind>,
    idle_kind: Option<AnimationKind>,
    active: Option<ActiveAnimation>,
    mouth_flap: Option<MouthFlapAnimation>,
    random_state: u64,
}

impl MotionState {
    pub fn new() -> Self {
        Self {
            next_kind: Some(AnimationKind::Bounce),
            idle_kind: None,
            active: None,
            mouth_flap: None,
            random_state: sampling::seed_from_system_time(),
        }
    }

    pub fn trigger(&mut self, now: Instant) {
        let kind = self.next_kind.unwrap_or(AnimationKind::Bounce);
        self.active = Some(Self::start_animation(kind, now, false));
        self.next_kind = Some(match kind {
            AnimationKind::Bounce => AnimationKind::SquashBounce,
            AnimationKind::SquashBounce => AnimationKind::Bounce,
            AnimationKind::IdleSink => AnimationKind::Bounce,
            AnimationKind::Shake => AnimationKind::Bounce,
        });
    }

    pub fn trigger_shake(
        &mut self,
        now: Instant,
        shake_amplitude_px: f32,
        shake_duration: Duration,
        shake_fps: u16,
    ) {
        self.active = Some(ActiveAnimation {
            kind: AnimationKind::Shake,
            idle: false,
            started_at: now,
            shake_amplitude_px: shake_amplitude_px.max(0.0),
            shake_seed: self.next_random_u64(),
            shake_duration: shake_duration.max(Duration::from_millis(1)),
            shake_frame_interval: sampling::frame_interval_from_fps(shake_fps),
        });
    }

    pub fn trigger_mouth_flap(&mut self, now: Instant, duration: Duration, fps: u16) {
        self.mouth_flap = Some(MouthFlapAnimation {
            started_at: now,
            duration: duration.max(Duration::from_millis(1)),
            frame_interval: sampling::frame_interval_from_fps(fps),
        });
    }

    pub fn set_always_idle_sink_enabled(&mut self, enabled: bool, now: Instant) {
        let was_idle_animation = self
            .active
            .is_some_and(|active| active.idle && self.idle_kind == Some(active.kind));
        self.idle_kind = enabled.then_some(AnimationKind::IdleSink);
        if enabled {
            if self.active.is_none() {
                self.active = Some(Self::start_animation(AnimationKind::IdleSink, now, true));
            }
        } else if was_idle_animation {
            self.active = None;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some() || self.mouth_flap.is_some()
    }

    pub fn mouth_flap_is_open(&mut self, now: Instant) -> Option<bool> {
        let mouth_flap = self.active_mouth_flap(now)?;
        let elapsed = now.duration_since(mouth_flap.started_at);
        let frame = elapsed.as_nanos() / mouth_flap.frame_interval.as_nanos().max(1);
        Some(frame % 2 == 0)
    }

    pub fn sample(
        &mut self,
        now: Instant,
        bounce: BounceAnimationConfig,
        squash_bounce: SquashBounceAnimationConfig,
        always_idle_sink: IdleSinkAnimationConfig,
    ) -> MotionTransform {
        self.ensure_idle_animation(now);
        let Some(active) = self.active else {
            return MotionTransform::identity();
        };

        let (duration, transform) = match active.kind {
            AnimationKind::Bounce => (
                Duration::from_millis(bounce.duration_ms.max(1)),
                sampling::sample_bounce(now, active.started_at, bounce),
            ),
            AnimationKind::SquashBounce => (
                Duration::from_millis(squash_bounce.duration_ms.max(1)),
                sampling::sample_squash_bounce(now, active.started_at, squash_bounce),
            ),
            AnimationKind::IdleSink => (
                Duration::from_millis(always_idle_sink.duration_ms.max(1)),
                sampling::sample_idle_sink(now, active.started_at, always_idle_sink),
            ),
            AnimationKind::Shake => (
                active.shake_duration,
                sampling::sample_shake(
                    now,
                    active.started_at,
                    active.shake_amplitude_px,
                    active.shake_seed,
                    active.shake_frame_interval,
                ),
            ),
        };

        if now.duration_since(active.started_at) >= duration {
            self.active = None;
            self.ensure_idle_animation(now);
            return MotionTransform::identity();
        }

        transform
    }

    pub fn repaint_after(
        &self,
        now: Instant,
        bounce: BounceAnimationConfig,
        squash_bounce: SquashBounceAnimationConfig,
        always_idle_sink: IdleSinkAnimationConfig,
    ) -> Option<Duration> {
        let active = match self.active {
            Some(active) => active,
            None => {
                let idle_repaint = self.idle_kind.map(|_| ANIMATION_FRAME_INTERVAL);
                return Self::min_repaint_after(idle_repaint, self.mouth_flap_repaint_after(now));
            }
        };
        let duration = match active.kind {
            AnimationKind::Bounce => Duration::from_millis(bounce.duration_ms.max(1)),
            AnimationKind::SquashBounce => Duration::from_millis(squash_bounce.duration_ms.max(1)),
            AnimationKind::IdleSink => Duration::from_millis(always_idle_sink.duration_ms.max(1)),
            AnimationKind::Shake => active.shake_duration,
        };
        let remaining = duration.saturating_sub(now.duration_since(active.started_at));
        if remaining.is_zero() {
            return Self::min_repaint_after(
                Some(Duration::ZERO),
                self.mouth_flap_repaint_after(now),
            );
        }

        let transform_repaint_after = Some(match active.kind {
            AnimationKind::Bounce | AnimationKind::SquashBounce | AnimationKind::IdleSink => {
                remaining.min(ANIMATION_FRAME_INTERVAL)
            }
            AnimationKind::Shake => remaining.min(sampling::next_shake_frame_after(
                now,
                active.started_at,
                active.shake_frame_interval,
            )),
        });
        Self::min_repaint_after(transform_repaint_after, self.mouth_flap_repaint_after(now))
    }

    fn next_random_u64(&mut self) -> u64 {
        self.random_state = sampling::splitmix64(self.random_state);
        self.random_state
    }

    fn ensure_idle_animation(&mut self, now: Instant) {
        if self.active.is_none() {
            self.active = self
                .idle_kind
                .map(|kind| Self::start_animation(kind, now, true));
        }
    }

    fn active_mouth_flap(&mut self, now: Instant) -> Option<MouthFlapAnimation> {
        let mouth_flap = self.mouth_flap?;
        if now.duration_since(mouth_flap.started_at) >= mouth_flap.duration {
            self.mouth_flap = None;
            return None;
        }
        Some(mouth_flap)
    }

    fn mouth_flap_repaint_after(&self, now: Instant) -> Option<Duration> {
        let mouth_flap = self.mouth_flap?;
        let elapsed = now.duration_since(mouth_flap.started_at);
        let remaining = mouth_flap.duration.saturating_sub(elapsed);
        if remaining.is_zero() {
            return Some(Duration::ZERO);
        }

        let next_frame = mouth_flap
            .frame_interval
            .checked_sub(Duration::from_nanos(
                (elapsed.as_nanos() % mouth_flap.frame_interval.as_nanos().max(1)) as u64,
            ))
            .unwrap_or(Duration::ZERO);
        Some(remaining.min(next_frame))
    }

    fn min_repaint_after(left: Option<Duration>, right: Option<Duration>) -> Option<Duration> {
        match (left, right) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        }
    }

    fn start_animation(kind: AnimationKind, now: Instant, idle: bool) -> ActiveAnimation {
        ActiveAnimation {
            kind,
            idle,
            started_at: now,
            shake_amplitude_px: 0.0,
            shake_seed: 0,
            shake_duration: Duration::ZERO,
            shake_frame_interval: Duration::ZERO,
        }
    }
}

#[cfg(test)]
impl MotionState {
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        Self {
            next_kind: Some(AnimationKind::Bounce),
            idle_kind: None,
            active: None,
            mouth_flap: None,
            random_state: seed,
        }
    }

    pub(crate) fn next_animation_name(&self) -> &'static str {
        match self.next_kind.unwrap_or(AnimationKind::Bounce) {
            AnimationKind::Bounce => "bounce",
            AnimationKind::SquashBounce => "squash_bounce",
            AnimationKind::IdleSink => "idle_sink",
            AnimationKind::Shake => "shake",
        }
    }
}

impl Default for MotionState {
    fn default() -> Self {
        Self::new()
    }
}
