#![cfg_attr(test, allow(dead_code))]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use mascot_render_core::{
    build_closed_eye_display_diff, default_eye_blink_targets, find_eye_blink_target,
    load_variation_spec, variation_spec_path, Core, DisplayDiff, MascotConfig, RenderRequest,
};

use crate::eye_blink_timing::EyeBlinkIntervalGenerator;

const CLOSED_MS: u64 = 200;

#[derive(Debug)]
pub(crate) struct EyeBlinkLoop {
    interval_generator: EyeBlinkIntervalGenerator,
    phase: BlinkPhase,
}

#[derive(Debug, Clone, Copy)]
enum BlinkPhase {
    Open { until: Instant },
    Closed { until: Instant },
}

impl EyeBlinkLoop {
    pub(crate) fn new(now: Instant) -> Self {
        let mut blink = Self {
            interval_generator: EyeBlinkIntervalGenerator::new(now),
            phase: BlinkPhase::Open { until: now },
        };
        blink.reset(now);
        blink
    }

    pub(crate) fn reset(&mut self, now: Instant) {
        self.phase = BlinkPhase::Open {
            until: now + self.next_open_duration(now),
        };
    }

    pub(crate) fn is_closed(&mut self, now: Instant) -> bool {
        self.advance(now);
        matches!(self.phase, BlinkPhase::Closed { .. })
    }

    pub(crate) fn repaint_after(&mut self, now: Instant, fallback: Duration) -> Duration {
        self.advance(now);
        self.current_deadline()
            .saturating_duration_since(now)
            .min(fallback)
    }

    pub(crate) fn current_median_ms(&self) -> f64 {
        self.interval_generator.current_median_ms()
    }

    fn advance(&mut self, now: Instant) {
        while now >= self.current_deadline() {
            self.phase = match self.phase {
                BlinkPhase::Open { .. } => BlinkPhase::Closed {
                    until: now + Duration::from_millis(CLOSED_MS),
                },
                BlinkPhase::Closed { .. } => BlinkPhase::Open {
                    until: now + self.next_open_duration(now),
                },
            };
        }
    }

    fn current_deadline(&self) -> Instant {
        match self.phase {
            BlinkPhase::Open { until } | BlinkPhase::Closed { until } => until,
        }
    }

    fn next_open_duration(&mut self, now: Instant) -> Duration {
        Duration::from_millis(self.interval_generator.next_interval_ms(now))
    }
}

pub(crate) fn render_closed_eye_png(core: &Core, config: &MascotConfig) -> Result<Option<PathBuf>> {
    let targets = default_eye_blink_targets();
    let psd_file_name = config
        .psd_path_in_zip
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            anyhow!(
                "invalid PSD file name in '{}'",
                config.psd_path_in_zip.display()
            )
        })?;
    let Some(target) = find_eye_blink_target(&targets, psd_file_name) else {
        return Ok(None);
    };

    let base_variation = load_current_display_diff(config);
    let document = core
        .inspect_psd(&config.zip_path, &config.psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect PSD '{}' for eye blink",
                config.psd_path_in_zip.display()
            )
        })?;
    let closed_display_diff = build_closed_eye_display_diff(&document, &base_variation, target)
        .map_err(|error| anyhow!(error))
        .with_context(|| {
            format!(
                "failed to build eye blink variation for '{}'",
                psd_file_name
            )
        })?;
    let rendered = core
        .render_png(RenderRequest {
            zip_path: config.zip_path.clone(),
            psd_path_in_zip: config.psd_path_in_zip.clone(),
            display_diff: closed_display_diff,
        })
        .with_context(|| format!("failed to render closed-eye PNG for '{}'", psd_file_name))?;

    Ok(Some(rendered.output_path))
}

fn load_current_display_diff(config: &MascotConfig) -> DisplayDiff {
    let active_variation_path = variation_spec_path(&config.png_path);
    load_variation_spec(
        &active_variation_path,
        &config.zip_path,
        &config.psd_path_in_zip,
    )
    .or_else(|| {
        config
            .display_diff_path
            .as_deref()
            .and_then(|path| load_variation_spec(path, &config.zip_path, &config.psd_path_in_zip))
    })
    .unwrap_or_default()
}

#[cfg(test)]
impl EyeBlinkLoop {
    pub(crate) fn new_for_test(now: Instant, seed: u64) -> Self {
        let mut blink = Self {
            interval_generator: EyeBlinkIntervalGenerator::new_for_test(now, seed),
            phase: BlinkPhase::Open { until: now },
        };
        blink.reset(now);
        blink
    }

    pub(crate) fn current_deadline_for_test(&self) -> Instant {
        self.current_deadline()
    }
}
