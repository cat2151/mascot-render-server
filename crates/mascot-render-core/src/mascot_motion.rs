use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};

const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

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

#[derive(Debug)]
pub struct MotionState {
    next_kind: Option<AnimationKind>,
    idle_kind: Option<AnimationKind>,
    active: Option<ActiveAnimation>,
    random_state: u64,
}

impl MotionState {
    pub fn new() -> Self {
        Self {
            next_kind: Some(AnimationKind::Bounce),
            idle_kind: None,
            active: None,
            random_state: seed_from_system_time(),
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
            shake_frame_interval: frame_interval_from_fps(shake_fps),
        });
    }

    pub fn set_always_bouncing(&mut self, enabled: bool, now: Instant) {
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
        self.active.is_some()
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
                sample_bounce(now, active.started_at, bounce),
            ),
            AnimationKind::SquashBounce => (
                Duration::from_millis(squash_bounce.duration_ms.max(1)),
                sample_squash_bounce(now, active.started_at, squash_bounce),
            ),
            AnimationKind::IdleSink => (
                Duration::from_millis(always_idle_sink.duration_ms.max(1)),
                sample_idle_sink(now, active.started_at, always_idle_sink),
            ),
            AnimationKind::Shake => (
                active.shake_duration,
                sample_shake(
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
            None if self.idle_kind.is_some() => return Some(ANIMATION_FRAME_INTERVAL),
            None => return None,
        };
        let duration = match active.kind {
            AnimationKind::Bounce => Duration::from_millis(bounce.duration_ms.max(1)),
            AnimationKind::SquashBounce => Duration::from_millis(squash_bounce.duration_ms.max(1)),
            AnimationKind::IdleSink => Duration::from_millis(always_idle_sink.duration_ms.max(1)),
            AnimationKind::Shake => active.shake_duration,
        };
        let remaining = duration.saturating_sub(now.duration_since(active.started_at));
        if remaining.is_zero() {
            return Some(Duration::ZERO);
        }

        Some(match active.kind {
            AnimationKind::Bounce | AnimationKind::SquashBounce | AnimationKind::IdleSink => {
                remaining.min(ANIMATION_FRAME_INTERVAL)
            }
            AnimationKind::Shake => remaining.min(next_shake_frame_after(
                now,
                active.started_at,
                active.shake_frame_interval,
            )),
        })
    }

    fn next_random_u64(&mut self) -> u64 {
        self.random_state = splitmix64(self.random_state);
        self.random_state
    }

    fn ensure_idle_animation(&mut self, now: Instant) {
        if self.active.is_none() {
            self.active = self
                .idle_kind
                .map(|kind| Self::start_animation(kind, now, true));
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

fn sample_bounce(
    now: Instant,
    started_at: Instant,
    config: BounceAnimationConfig,
) -> MotionTransform {
    let t = animation_progress(now, started_at, config.duration_ms);
    let envelope = (1.0 - t).powf(config.damping.max(0.0));
    let phase = std::f32::consts::PI * config.cycles.max(0.1) * t;
    let vertical = phase.sin().abs() * envelope;

    MotionTransform {
        offset_x: 0.0,
        offset_y: -config.amplitude_px.max(0.0) * vertical,
        scale_x: 1.0,
        scale_y: 1.0,
    }
}

fn sample_squash_bounce(
    now: Instant,
    started_at: Instant,
    config: SquashBounceAnimationConfig,
) -> MotionTransform {
    let t = animation_progress(now, started_at, config.duration_ms);
    let envelope = (1.0 - t).powf(config.damping.max(0.0));
    let phase = std::f32::consts::PI * config.cycles.max(0.1) * t;
    let vertical = phase.sin().abs() * envelope;
    let landing = (phase.cos().abs() * envelope).powf(1.3);

    MotionTransform {
        offset_x: 0.0,
        offset_y: -config.amplitude_px.max(0.0) * vertical,
        scale_x: 1.0 + config.squash_amount.max(0.0) * landing,
        scale_y: 1.0 - config.stretch_amount.max(0.0) * landing,
    }
}

fn sample_idle_sink(
    now: Instant,
    started_at: Instant,
    config: IdleSinkAnimationConfig,
) -> MotionTransform {
    let t = animation_progress(now, started_at, config.duration_ms);
    let sink = phase_pulse(t, 0.0, 0.25, 0.5);
    let lift = phase_pulse(t, 0.5, 0.75, 1.0);

    MotionTransform {
        offset_x: 0.0,
        offset_y: config.amplitude_px.max(0.0) * (sink - lift),
        scale_x: 1.0 + config.sink_amount.max(0.0) * sink
            - config.lift_amount.max(0.0) * lift * 0.35,
        scale_y: 1.0 - config.sink_amount.max(0.0) * sink + config.lift_amount.max(0.0) * lift,
    }
}

fn sample_shake(
    now: Instant,
    started_at: Instant,
    shake_amplitude_px: f32,
    shake_seed: u64,
    shake_frame_interval: Duration,
) -> MotionTransform {
    let frame_index =
        now.duration_since(started_at).as_millis() / shake_frame_interval.as_millis().max(1);
    let frame_seed = shake_seed ^ (frame_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    MotionTransform {
        offset_x: shake_amplitude_px * random_signed_unit(frame_seed),
        offset_y: shake_amplitude_px * random_signed_unit(frame_seed.rotate_left(32)),
        scale_x: 1.0,
        scale_y: 1.0,
    }
}

fn animation_progress(now: Instant, started_at: Instant, duration_ms: u64) -> f32 {
    let duration = Duration::from_millis(duration_ms.max(1));
    let elapsed = now.duration_since(started_at);
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn phase_pulse(t: f32, start: f32, peak: f32, end: f32) -> f32 {
    if t <= start || t >= end {
        return 0.0;
    }
    if t < peak {
        return smoothstep((t - start) / (peak - start).max(f32::EPSILON));
    }
    smoothstep((end - t) / (end - peak).max(f32::EPSILON))
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn next_shake_frame_after(
    now: Instant,
    started_at: Instant,
    shake_frame_interval: Duration,
) -> Duration {
    let elapsed = now.duration_since(started_at);
    let elapsed_ms = elapsed.as_millis();
    let interval_ms = shake_frame_interval.as_millis().max(1);
    let next_frame_ms = ((elapsed_ms / interval_ms) + 1) * interval_ms;
    Duration::from_millis(next_frame_ms.saturating_sub(elapsed_ms) as u64)
}

fn frame_interval_from_fps(fps: u16) -> Duration {
    let fps = u64::from(fps.max(1));
    Duration::from_millis((1_000 / fps).max(1))
}

fn random_signed_unit(seed: u64) -> f32 {
    let normalized = (splitmix64(seed) as f64 / u64::MAX as f64) as f32;
    normalized * 2.0 - 1.0
}

fn seed_from_system_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0xA5A5_5A5A_DEAD_BEEF)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
impl MotionState {
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        Self {
            next_kind: Some(AnimationKind::Bounce),
            idle_kind: None,
            active: None,
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
