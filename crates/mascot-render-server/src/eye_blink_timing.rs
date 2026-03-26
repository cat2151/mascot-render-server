#![cfg_attr(test, allow(dead_code))]

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mascot_render_core::SquashBounceAnimationConfig;

const DEFAULT_MEDIAN_MS: f64 = 3600.0;
const DEFAULT_LOG_SIGMA: f64 = 0.42;
const DEFAULT_MIN_MS: u64 = 1000;
const DEFAULT_MAX_MS: u64 = 8000;
const DEFAULT_DRIFT_RATIO_LIMIT: f64 = 0.20;
const ALWAYS_BOUNCE_DURATION_SCALE_MIN: f64 = 1.0 - DEFAULT_DRIFT_RATIO_LIMIT;
const ALWAYS_BOUNCE_DURATION_SCALE_MAX: f64 = 1.0 + DEFAULT_DRIFT_RATIO_LIMIT;
const DRIFT_MIN_SECS: f64 = 5.0;
const DRIFT_MAX_SECS: f64 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct EyeBlinkTimingConfig {
    pub(crate) median_ms: f64,
    pub(crate) log_sigma: f64,
    pub(crate) min_ms: u64,
    pub(crate) max_ms: u64,
}

impl Default for EyeBlinkTimingConfig {
    fn default() -> Self {
        Self {
            median_ms: DEFAULT_MEDIAN_MS,
            log_sigma: DEFAULT_LOG_SIGMA,
            min_ms: DEFAULT_MIN_MS,
            max_ms: DEFAULT_MAX_MS,
        }
    }
}

#[derive(Debug)]
pub(crate) struct EyeBlinkIntervalGenerator {
    rng_state: u64,
    config: EyeBlinkTimingConfig,
    drift_ratio: f64,
    drift_timescale_secs: f64,
    last_drift_update_at: Instant,
}

impl EyeBlinkIntervalGenerator {
    pub(crate) fn new(now: Instant) -> Self {
        Self::new_with_seed(now, seed_from_clock(), EyeBlinkTimingConfig::default())
    }

    pub(crate) fn next_interval_ms(&mut self, now: Instant) -> u64 {
        self.advance_drift(now);
        let z = self.sample_standard_normal();
        let median_ms = self.current_median_ms_value();
        let log_sigma = self.config.log_sigma.max(0.0);
        let interval_ms = (median_ms.ln() + z * log_sigma).exp();
        clamp_interval_ms(interval_ms, self.config.min_ms, self.config.max_ms)
    }

    fn advance_drift(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_drift_update_at);
        self.last_drift_update_at = now;
        let dt_secs = elapsed.as_secs_f64();
        if dt_secs <= f64::EPSILON {
            return;
        }

        let decay = (-dt_secs / self.drift_timescale_secs).exp();
        let noise_scale = (1.0 - decay * decay).max(0.0).sqrt() * 0.11;
        self.drift_ratio = (self.drift_ratio * decay + self.sample_standard_normal() * noise_scale)
            .clamp(-DEFAULT_DRIFT_RATIO_LIMIT, DEFAULT_DRIFT_RATIO_LIMIT);
        self.drift_timescale_secs = self.sample_drift_timescale_secs();
    }

    fn current_median_ms_value(&self) -> f64 {
        self.config.median_ms * (1.0 + self.drift_ratio)
    }

    fn sample_drift_timescale_secs(&mut self) -> f64 {
        let uniform = self.sample_unit_f64();
        DRIFT_MIN_SECS + (DRIFT_MAX_SECS - DRIFT_MIN_SECS) * uniform
    }

    fn sample_standard_normal(&mut self) -> f64 {
        let u1 = (1.0 - self.sample_unit_f64()).clamp(f64::MIN_POSITIVE, 1.0);
        let u2 = self.sample_unit_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    fn sample_unit_f64(&mut self) -> f64 {
        let bits = self.next_u64() >> 11;
        bits as f64 / ((1u64 << 53) as f64)
    }

    fn next_u64(&mut self) -> u64 {
        if self.rng_state == 0 {
            self.rng_state = 0x9e37_79b9_7f4a_7c15;
        }
        self.rng_state ^= self.rng_state >> 12;
        self.rng_state ^= self.rng_state << 25;
        self.rng_state ^= self.rng_state >> 27;
        self.rng_state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(now: Instant, seed: u64) -> Self {
        Self::new_with_seed(now, seed, EyeBlinkTimingConfig::default())
    }

    fn new_with_seed(now: Instant, seed: u64, config: EyeBlinkTimingConfig) -> Self {
        let mut generator = Self {
            rng_state: seed,
            config,
            drift_ratio: 0.0,
            drift_timescale_secs: DRIFT_MIN_SECS,
            last_drift_update_at: now,
        };
        generator.drift_timescale_secs = generator.sample_drift_timescale_secs();
        generator
    }

    #[cfg(test)]
    pub(crate) fn current_median_ms_for_test(&self) -> f64 {
        self.current_median_ms_value()
    }

    pub(crate) fn current_median_ms(&self) -> f64 {
        self.current_median_ms_value()
    }
}

fn clamp_interval_ms(interval_ms: f64, min_ms: u64, max_ms: u64) -> u64 {
    interval_ms.round().clamp(min_ms as f64, max_ms as f64) as u64
}

pub(crate) fn always_squash_bounce_for_blink_median(
    config: SquashBounceAnimationConfig,
    blink_median_ms: f64,
) -> SquashBounceAnimationConfig {
    // Keep the always_bouncing tempo aligned with the blink median drift range (±20%).
    let duration_scale = (blink_median_ms / DEFAULT_MEDIAN_MS).clamp(
        ALWAYS_BOUNCE_DURATION_SCALE_MIN,
        ALWAYS_BOUNCE_DURATION_SCALE_MAX,
    ) as f32;
    SquashBounceAnimationConfig {
        duration_ms: ((config.duration_ms as f32) * duration_scale)
            .round()
            .max(1.0) as u64,
        ..config
    }
}

fn seed_from_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(0x6eed_0e9d_a4d9_4a4f)
}
