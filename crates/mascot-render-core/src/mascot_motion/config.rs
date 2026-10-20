use serde::{Deserialize, Serialize};

const DEFAULT_ALWAYS_BEND_AMPLITUDE_RATIO: f32 = 0.0075;

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
#[serde(default, deny_unknown_fields)]
pub struct IdleSinkAnimationConfig {
    pub algorithm: IdleAlgorithm,
    pub duration_ms: u64,
    pub sink_amount: f32,
    pub lift_amount: f32,
}

impl Default for IdleSinkAnimationConfig {
    fn default() -> Self {
        Self {
            algorithm: IdleAlgorithm::IdleSink,
            duration_ms: 2200,
            sink_amount: 0.0015,
            lift_amount: 0.0015,
        }
    }
}

impl IdleSinkAnimationConfig {
    pub fn default_for_always_bouncing() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlwaysBendConfig {
    pub enabled: bool,
    pub amplitude_ratio: f32,
}

impl Default for AlwaysBendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            amplitude_ratio: DEFAULT_ALWAYS_BEND_AMPLITUDE_RATIO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BendConfig {
    pub amplitude_ratio: f32,
}

impl Default for BendConfig {
    fn default() -> Self {
        Self {
            amplitude_ratio: DEFAULT_ALWAYS_BEND_AMPLITUDE_RATIO,
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
    pub fn default_for_musical() -> Self {
        Self {
            algorithm: SquashAlgorithm::SquashStretch,
            duration_ms: 500,
            amplitude_px: 3.0,
            cycles: 1.0,
            damping: 2.0,
            squash_amount: 0.1,
            stretch_amount: 0.05,
        }
    }

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

impl SquashBounceAnimationConfig {
    /// Samples the same motion used for the click reaction, without rescaling time.
    pub fn sample(self, elapsed: std::time::Duration) -> super::MotionTransform {
        if elapsed >= std::time::Duration::from_millis(self.duration_ms.max(1)) {
            return super::MotionTransform::identity();
        }
        super::sampling::sample_squash_bounce(elapsed, self)
    }
}
