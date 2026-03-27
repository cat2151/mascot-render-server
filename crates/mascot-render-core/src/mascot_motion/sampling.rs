use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{
    BounceAnimationConfig, IdleSinkAnimationConfig, MotionTransform, SquashBounceAnimationConfig,
    IDLE_SINK_LIFT_SCALE_X_RATIO,
};

pub(super) fn sample_bounce(
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

pub(super) fn sample_squash_bounce(
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

pub(super) fn sample_idle_sink(
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
            - config.lift_amount.max(0.0) * lift * IDLE_SINK_LIFT_SCALE_X_RATIO,
        scale_y: 1.0 - config.sink_amount.max(0.0) * sink + config.lift_amount.max(0.0) * lift,
    }
}

pub(super) fn sample_shake(
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

pub(super) fn next_shake_frame_after(
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

pub(super) fn frame_interval_from_fps(fps: u16) -> Duration {
    let fps = u64::from(fps.max(1));
    Duration::from_millis((1_000 / fps).max(1))
}

fn random_signed_unit(seed: u64) -> f32 {
    let normalized = (splitmix64(seed) as f64 / u64::MAX as f64) as f32;
    normalized * 2.0 - 1.0
}

pub(super) fn seed_from_system_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0xA5A5_5A5A_DEAD_BEEF)
}

pub(super) fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
