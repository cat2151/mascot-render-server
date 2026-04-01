#![cfg_attr(test, allow(dead_code))]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use mascot_render_core::{
    auto_generate_eye_blink_target, build_closed_eye_display_diff, load_variation_spec,
    variation_spec_path, Core, DisplayDiff, MascotConfig, PsdDocument, RenderRequest,
};

use crate::eye_blink_timing::EyeBlinkIntervalGenerator;

const CLOSED_MS: u64 = 200;
static EYE_BLINK_SKIP_LOGGED_PSDS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

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

    pub(crate) fn new_with_seed_and_elapsed(now: Instant, seed: u64, elapsed: Duration) -> Self {
        let started_at = now.checked_sub(elapsed).unwrap_or(now);
        let mut blink = Self {
            interval_generator: EyeBlinkIntervalGenerator::new_with_seed(started_at, seed),
            phase: BlinkPhase::Open { until: started_at },
        };
        blink.reset(started_at);
        blink.advance(now);
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

    /// Returns the time until the next blink phase transition without applying
    /// an external repaint cap.
    pub(crate) fn deadline_after(&mut self, now: Instant) -> Duration {
        self.advance(now);
        self.current_deadline().saturating_duration_since(now)
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
    render_closed_eye_png_with_display_diff(
        core,
        &config.zip_path,
        &config.psd_path_in_zip,
        &load_current_display_diff(config),
    )
}

pub(crate) fn render_closed_eye_png_with_display_diff(
    core: &Core,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    base_variation: &DisplayDiff,
) -> Result<Option<PathBuf>> {
    let psd_file_name = psd_path_in_zip
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("invalid PSD file name in '{}'", psd_path_in_zip.display()))?;
    let document = core
        .inspect_psd(zip_path, psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect PSD '{}' for eye blink",
                psd_path_in_zip.display()
            )
        })?;
    let Some(closed_display_diff) = build_closed_eye_display_diff_with_document(
        zip_path,
        psd_path_in_zip,
        &document,
        base_variation,
    )?
    else {
        return Ok(None);
    };
    render_closed_eye_png_with_closed_display_diff(
        core,
        zip_path,
        psd_path_in_zip,
        psd_file_name,
        closed_display_diff,
    )
}

/// Builds the closed-eye display diff from an already inspected PSD document so
/// callers can reuse the inspection result.
pub(crate) fn build_closed_eye_display_diff_with_document(
    zip_path: &Path,
    psd_path_in_zip: &Path,
    document: &PsdDocument,
    base_variation: &DisplayDiff,
) -> Result<Option<DisplayDiff>> {
    let target = match auto_generate_eye_blink_target(document, base_variation) {
        Ok(target) => target,
        Err(error) => {
            log_eye_blink_auto_generation_skip_once(zip_path, psd_path_in_zip, &error);
            return Ok(None);
        }
    };
    let psd_file_name = psd_path_in_zip
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("invalid PSD file name in '{}'", psd_path_in_zip.display()))?;
    let closed_display_diff = build_closed_eye_display_diff(document, base_variation, &target)
        .map_err(|error| anyhow!(error))
        .with_context(|| {
            format!(
                "failed to build eye blink variation for '{}'",
                psd_file_name
            )
        })?;
    Ok(Some(closed_display_diff))
}

// Renders the closed-eye PNG from a prebuilt closed-eye display diff.
fn render_closed_eye_png_with_closed_display_diff(
    core: &Core,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    psd_file_name: &str,
    closed_display_diff: DisplayDiff,
) -> Result<Option<PathBuf>> {
    let rendered = core
        .render_png(RenderRequest {
            zip_path: zip_path.to_path_buf(),
            psd_path_in_zip: psd_path_in_zip.to_path_buf(),
            display_diff: closed_display_diff,
        })
        .with_context(|| format!("failed to render closed-eye PNG for '{}'", psd_file_name))?;

    Ok(Some(rendered.output_path))
}

fn log_eye_blink_auto_generation_skip_once(zip_path: &Path, psd_path_in_zip: &Path, error: &str) {
    let key = format!("{}::{}", zip_path.display(), psd_path_in_zip.display());
    let logged_psds = EYE_BLINK_SKIP_LOGGED_PSDS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut logged_psds = logged_psds
        .lock()
        .expect("eye blink skip log state should be lockable");
    if !logged_psds.insert(key) {
        return;
    }
    eprintln!(
        "Eye blink auto-generation skipped: zip_path={} psd_path_in_zip={} reason={}",
        zip_path.display(),
        psd_path_in_zip.display(),
        error
    );
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
