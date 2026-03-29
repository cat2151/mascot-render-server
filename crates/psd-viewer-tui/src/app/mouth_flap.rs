use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use mascot_render_core::{
    auto_generate_mouth_flap_target, describe_mouth_flap_auto_generation_failure,
    resolve_mouth_flap_rows, variation_spec_path, DisplayDiff, PsdDocument, PsdEntry,
    RenderRequest,
};

use super::support::current_preview_status;
use super::App;
use crate::display_diff_state::{resolve_layer_rows, toggle_layer_override};

const MOUTH_FLAP_DURATION: Duration = Duration::from_secs(5);
const MOUTH_FLAP_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(crate) struct MouthFlapAnimation {
    base_preview_png_path: Option<PathBuf>,
    base_variation_spec_path: Option<PathBuf>,
    frames: [MouthFlapFrame; 2],
    active_frame_index: usize,
    next_switch_at: Instant,
    ends_at: Instant,
}

#[derive(Debug, Clone)]
struct MouthFlapFrame {
    label: String,
    preview_png_path: PathBuf,
    variation_spec_path: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct MouthFlapFrameContext<'a> {
    zip_path: &'a Path,
    psd_path_in_zip: &'a Path,
    psd_entry: &'a PsdEntry,
    document: &'a PsdDocument,
    base_variation: &'a DisplayDiff,
}

impl App {
    pub(crate) fn start_mouth_flap_preview(&mut self) -> bool {
        let outcome = self.start_mouth_flap_preview_inner();
        match outcome {
            Ok(animation) => {
                let frame = animation.current_frame();
                self.clear_preview_animations();
                self.clear_log_overlay();
                self.current_preview_png_path = Some(frame.preview_png_path.clone());
                self.current_variation_spec_path = frame.variation_spec_path.clone();
                self.status = format!(
                    "Mouth flap preview: {} / {} (5s, 250ms)",
                    animation.frames[0].label.as_str(),
                    animation.frames[1].label.as_str()
                );
                self.mouth_flap = Some(animation);
                true
            }
            Err(error) => {
                self.status = format!("Mouth flap preview unavailable: {error}");
                false
            }
        }
    }

    pub(crate) fn process_mouth_flap_animation(&mut self) -> bool {
        let Some(mut animation) = self.mouth_flap.take() else {
            return false;
        };

        let now = Instant::now();
        if animation.is_finished(now) {
            self.current_preview_png_path = animation.base_preview_png_path;
            self.current_variation_spec_path = animation.base_variation_spec_path;
            self.status = format!(
                "Mouth flap preview finished. {}",
                current_preview_status(
                    self.current_preview_png_path.as_deref(),
                    self.current_variation_spec_path.as_deref()
                )
            );
            return true;
        }

        if animation.advance(now) {
            let frame = animation.current_frame();
            self.current_preview_png_path = Some(frame.preview_png_path.clone());
            self.current_variation_spec_path = frame.variation_spec_path.clone();
            self.status = format!("Mouth flap preview: {}", frame.label);
            self.mouth_flap = Some(animation);
            return true;
        }

        self.mouth_flap = Some(animation);
        false
    }

    pub(crate) fn event_poll_timeout(&self, default_timeout: Duration) -> Duration {
        let mouth_flap_timeout = self
            .mouth_flap
            .as_ref()
            .map(|animation| animation.poll_timeout(default_timeout))
            .unwrap_or(default_timeout);
        let eye_blink_timeout = self
            .eye_blink
            .as_ref()
            .map(|animation| animation.poll_timeout(default_timeout))
            .unwrap_or(default_timeout);

        mouth_flap_timeout.min(eye_blink_timeout)
    }

    fn start_mouth_flap_preview_inner(&mut self) -> Result<MouthFlapAnimation, String> {
        let selected_psd_path = self
            .selected_psd_entry()
            .map(|entry| entry.path.clone())
            .ok_or_else(|| "no PSD selected".to_string())?;
        let document = self
            .current_psd_document
            .clone()
            .ok_or_else(|| "selected PSD document is not loaded".to_string())?;
        let (zip_path, extracted_dir) = self
            .selected_zip_entry()
            .map(|zip| (zip.zip_path.clone(), zip.extracted_dir.clone()))
            .ok_or_else(|| "no ZIP selected".to_string())?;
        let psd_entry = self
            .selected_psd_entry()
            .cloned()
            .ok_or_else(|| "no PSD entry selected".to_string())?;
        let psd_path_in_zip = super::support::psd_path_in_zip(
            &selected_psd_path,
            &extracted_dir,
            &document.psd_path_in_zip,
        );
        let base_variation = self
            .variations
            .get(&selected_psd_path)
            .cloned()
            .unwrap_or_default();
        let target = match auto_generate_mouth_flap_target(&document, &base_variation) {
            Ok(target) => target,
            Err(error) => {
                let diagnostic = format_auto_mouth_flap_generation_failure_log(
                    &psd_entry.file_name,
                    &describe_mouth_flap_auto_generation_failure(&document, &base_variation),
                    &error,
                );
                self.show_log_overlay(diagnostic);
                return Err(format!(
                    "selected PSD '{}' does not support automatic mouth flap preview: {error}",
                    psd_entry.file_name
                ));
            }
        };
        let resolved = resolve_mouth_flap_rows(&document, &base_variation, &target)?;
        let frame_context = MouthFlapFrameContext {
            zip_path: &zip_path,
            psd_path_in_zip: &psd_path_in_zip,
            psd_entry: &psd_entry,
            document: &document,
            base_variation: &base_variation,
        };
        let frames = [
            self.build_mouth_flap_frame(
                frame_context,
                resolved.open_row_index,
                &resolved.open_label,
            )?,
            self.build_mouth_flap_frame(
                frame_context,
                resolved.closed_row_index,
                &resolved.closed_label,
            )?,
        ];

        Ok(MouthFlapAnimation::new(
            self.current_preview_png_path.clone(),
            self.current_variation_spec_path.clone(),
            frames,
        ))
    }

    fn build_mouth_flap_frame(
        &self,
        context: MouthFlapFrameContext<'_>,
        row_index: usize,
        label: &str,
    ) -> Result<MouthFlapFrame, String> {
        let variation =
            ensure_named_row_visible(context.base_variation, context.document, row_index, label)?;
        let (preview_png_path, variation_spec_path) =
            self.render_preview_for_spec(context, &variation)?;

        Ok(MouthFlapFrame {
            label: label.to_string(),
            preview_png_path,
            variation_spec_path,
        })
    }

    fn render_preview_for_spec(
        &self,
        context: MouthFlapFrameContext<'_>,
        display_diff: &DisplayDiff,
    ) -> Result<(PathBuf, Option<PathBuf>), String> {
        if display_diff.is_default() {
            let preview_png_path =
                context.psd_entry.rendered_png_path.clone().ok_or_else(|| {
                    format!("default PNG is missing for {}", context.psd_entry.file_name)
                })?;
            return Ok((preview_png_path, None));
        }

        let rendered = self
            .core
            .render_png(RenderRequest {
                zip_path: context.zip_path.to_path_buf(),
                psd_path_in_zip: context.psd_path_in_zip.to_path_buf(),
                display_diff: display_diff.clone(),
            })
            .map_err(|error| error.to_string())?;

        Ok((
            rendered.output_path.clone(),
            Some(variation_spec_path(&rendered.output_path)),
        ))
    }
}

fn format_auto_mouth_flap_generation_failure_log(
    psd_file_name: &str,
    diagnostic: &str,
    error: &str,
) -> String {
    format!(
        "Failed to auto-generate mouth flap target for PSD '{psd_file_name}'.\nmouth-group diagnostics:\n{}\nreason: {error}",
        diagnostic,
    )
}

fn ensure_named_row_visible(
    base_variation: &DisplayDiff,
    document: &PsdDocument,
    row_index: usize,
    label: &str,
) -> Result<DisplayDiff, String> {
    let mut variation = base_variation.clone();
    let rows = resolve_layer_rows(document, &variation);
    if rows.get(row_index).is_none() {
        return Err(format!("mouth flap target '{}' row is missing", label));
    }
    if rows[row_index].visible {
        return Ok(variation);
    }

    if !toggle_layer_override(&mut variation, document, row_index) {
        return Err(format!("failed to activate mouth flap target '{}'", label));
    }

    let rows = resolve_layer_rows(document, &variation);
    if rows.get(row_index).is_some_and(|row| row.visible) {
        return Ok(variation);
    }

    Err(format!(
        "mouth flap target '{}' stayed hidden after toggle; parent group may be hidden",
        label
    ))
}

impl MouthFlapAnimation {
    fn new(
        base_preview_png_path: Option<PathBuf>,
        base_variation_spec_path: Option<PathBuf>,
        frames: [MouthFlapFrame; 2],
    ) -> Self {
        let now = Instant::now();
        Self {
            base_preview_png_path,
            base_variation_spec_path,
            frames,
            active_frame_index: 0,
            next_switch_at: now + MOUTH_FLAP_INTERVAL,
            ends_at: now + MOUTH_FLAP_DURATION,
        }
    }

    fn current_frame(&self) -> &MouthFlapFrame {
        &self.frames[self.active_frame_index]
    }

    fn is_finished(&self, now: Instant) -> bool {
        now >= self.ends_at
    }

    fn advance(&mut self, now: Instant) -> bool {
        if now < self.next_switch_at {
            return false;
        }

        while now >= self.next_switch_at {
            self.active_frame_index = (self.active_frame_index + 1) % self.frames.len();
            self.next_switch_at += MOUTH_FLAP_INTERVAL;
        }
        true
    }

    fn poll_timeout(&self, default_timeout: Duration) -> Duration {
        let now = Instant::now();
        if self.is_finished(now) {
            return Duration::ZERO;
        }

        self.next_switch_at
            .min(self.ends_at)
            .saturating_duration_since(now)
            .min(default_timeout)
    }
}

#[cfg(test)]
impl MouthFlapAnimation {
    pub(crate) fn new_for_test(frames: [PathBuf; 2], now: Instant) -> Self {
        Self::new_with_labels_for_test(frames, ["ほあー".to_string(), "むふ".to_string()], now)
    }

    pub(crate) fn new_with_labels_for_test(
        frames: [PathBuf; 2],
        labels: [String; 2],
        now: Instant,
    ) -> Self {
        Self {
            base_preview_png_path: None,
            base_variation_spec_path: None,
            frames: [
                MouthFlapFrame {
                    label: labels[0].clone(),
                    preview_png_path: frames[0].clone(),
                    variation_spec_path: None,
                },
                MouthFlapFrame {
                    label: labels[1].clone(),
                    preview_png_path: frames[1].clone(),
                    variation_spec_path: None,
                },
            ],
            active_frame_index: 0,
            next_switch_at: now + MOUTH_FLAP_INTERVAL,
            ends_at: now + MOUTH_FLAP_DURATION,
        }
    }

    pub(crate) fn advance_for_test(&mut self, now: Instant) -> bool {
        self.advance(now)
    }

    pub(crate) fn current_frame_label_for_test(&self) -> &str {
        &self.current_frame().label
    }

    pub(crate) fn is_finished_for_test(&self, now: Instant) -> bool {
        self.is_finished(now)
    }
}
