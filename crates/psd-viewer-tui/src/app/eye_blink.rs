use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use mascot_render_core::{
    auto_generate_eye_blink_target_with_keywords, resolve_eye_blink_rows, variation_spec_path,
    DisplayDiff, EyeBlinkRows, PsdDocument, PsdEntry, RenderRequest,
};

use super::support::current_preview_status;
use super::App;
use crate::display_diff_state::{resolve_layer_rows, toggle_layer_override, LayerRow};

const EYE_BLINK_DURATION: Duration = Duration::from_secs(5);
const EYE_BLINK_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(crate) struct EyeBlinkAnimation {
    base_preview_png_path: Option<PathBuf>,
    base_variation_spec_path: Option<PathBuf>,
    frames: [EyeBlinkFrame; 2],
    active_frame_index: usize,
    next_switch_at: Instant,
    ends_at: Instant,
}

#[derive(Debug, Clone)]
struct EyeBlinkFrame {
    label: String,
    preview_png_path: PathBuf,
    variation_spec_path: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct EyeBlinkFrameContext<'a> {
    zip_path: &'a Path,
    psd_path_in_zip: &'a Path,
    psd_entry: &'a PsdEntry,
    document: &'a PsdDocument,
    base_variation: &'a DisplayDiff,
}

impl App {
    pub(crate) fn clear_preview_animations(&mut self) {
        self.eye_blink = None;
        self.mouth_flap = None;
    }

    pub(crate) fn is_preview_animation_active(&self) -> bool {
        self.eye_blink.is_some() || self.mouth_flap.is_some()
    }

    pub(crate) fn start_eye_blink_preview(&mut self) -> bool {
        let outcome = self.start_eye_blink_preview_inner();
        match outcome {
            Ok((animation, resolved)) => {
                let frame = animation.current_frame();
                self.clear_preview_animations();
                self.current_preview_png_path = Some(frame.preview_png_path.clone());
                self.current_variation_spec_path = frame.variation_spec_path.clone();
                self.status = format!(
                    "Eye blink preview: {} / {} (5s, 250ms)",
                    resolved.open_label, resolved.closed_label
                );
                self.eye_blink = Some(animation);
                true
            }
            Err(error) => {
                self.status = format!("Eye blink preview unavailable: {error}");
                eprintln!("{error:#}");
                false
            }
        }
    }

    pub(crate) fn process_eye_blink_animation(&mut self) -> bool {
        let Some(mut animation) = self.eye_blink.take() else {
            return false;
        };

        let now = Instant::now();
        if animation.is_finished(now) {
            self.current_preview_png_path = animation.base_preview_png_path;
            self.current_variation_spec_path = animation.base_variation_spec_path;
            self.status = format!(
                "Eye blink preview finished. {}",
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
            self.status = format!("Eye blink preview: {}", frame.label);
            self.eye_blink = Some(animation);
            return true;
        }

        self.eye_blink = Some(animation);
        false
    }

    fn start_eye_blink_preview_inner(&self) -> Result<(EyeBlinkAnimation, EyeBlinkRows), String> {
        let selected_psd_path = self
            .selected_psd_entry()
            .map(|entry| entry.path.clone())
            .ok_or_else(|| "no PSD selected".to_string())?;
        let document = self
            .current_psd_document
            .as_ref()
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
        let target = auto_generate_eye_blink_target_with_keywords(
            document,
            &base_variation,
            &self.eye_blink_preferred_open_layer_names,
            &self.eye_blink_closed_layer_keywords,
        )
        .map_err(|error| {
            let layer_rows = resolve_layer_rows(document, &base_variation);
            eprintln!(
                "{}",
                format_auto_eye_blink_generation_failure_log(
                    &psd_entry.file_name,
                    &layer_rows,
                    &error
                )
            );
            format!(
                "selected PSD '{}' does not support automatic eye blink preview: {error}",
                psd_entry.file_name
            )
        })?;
        let resolved = resolve_eye_blink_rows(document, &base_variation, &target)?;
        let frame_context = EyeBlinkFrameContext {
            zip_path: &zip_path,
            psd_path_in_zip: &psd_path_in_zip,
            psd_entry: &psd_entry,
            document,
            base_variation: &base_variation,
        };
        let frames = [
            self.build_eye_blink_frame(
                frame_context,
                resolved.open_row_index,
                &resolved.open_label,
            )?,
            self.build_eye_blink_frame(
                frame_context,
                resolved.closed_row_index,
                &resolved.closed_label,
            )?,
        ];

        Ok((
            EyeBlinkAnimation::new(
                self.current_preview_png_path.clone(),
                self.current_variation_spec_path.clone(),
                frames,
            ),
            resolved,
        ))
    }

    fn build_eye_blink_frame(
        &self,
        context: EyeBlinkFrameContext<'_>,
        row_index: usize,
        label: &str,
    ) -> Result<EyeBlinkFrame, String> {
        let variation =
            ensure_named_row_visible(context.base_variation, context.document, row_index, label)?;
        let (preview_png_path, variation_spec_path) =
            self.render_eye_blink_preview_for_spec(context, &variation)?;

        Ok(EyeBlinkFrame {
            label: label.to_string(),
            preview_png_path,
            variation_spec_path,
        })
    }

    fn render_eye_blink_preview_for_spec(
        &self,
        context: EyeBlinkFrameContext<'_>,
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

fn format_auto_eye_blink_generation_failure_log(
    psd_file_name: &str,
    layer_rows: &[LayerRow],
    error: &str,
) -> String {
    format!(
        "Failed to auto-generate eye blink target for PSD '{psd_file_name}'.\nlayer list:\n{}\nreason: {error}",
        format_eye_blink_layer_rows(layer_rows)
    )
}

fn format_eye_blink_layer_rows(layer_rows: &[LayerRow]) -> String {
    layer_rows
        .iter()
        .enumerate()
        .map(|(index, row)| format!("  - [{}] {}", index, row.display_label()))
        .collect::<Vec<_>>()
        .join("\n")
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
        return Err(format!("eye blink target '{}' row is missing", label));
    }
    if rows[row_index].visible {
        return Ok(variation);
    }

    if !toggle_layer_override(&mut variation, document, row_index) {
        return Err(format!("failed to activate eye blink target '{}'", label));
    }

    let rows = resolve_layer_rows(document, &variation);
    if rows.get(row_index).is_some_and(|row| row.visible) {
        return Ok(variation);
    }

    Err(format!(
        "eye blink target '{}' stayed hidden after toggle; parent group may be hidden",
        label
    ))
}

impl EyeBlinkAnimation {
    fn new(
        base_preview_png_path: Option<PathBuf>,
        base_variation_spec_path: Option<PathBuf>,
        frames: [EyeBlinkFrame; 2],
    ) -> Self {
        let now = Instant::now();
        Self {
            base_preview_png_path,
            base_variation_spec_path,
            frames,
            active_frame_index: 0,
            next_switch_at: now + EYE_BLINK_INTERVAL,
            ends_at: now + EYE_BLINK_DURATION,
        }
    }

    fn current_frame(&self) -> &EyeBlinkFrame {
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
            self.next_switch_at += EYE_BLINK_INTERVAL;
        }
        true
    }

    pub(super) fn poll_timeout(&self, default_timeout: Duration) -> Duration {
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
