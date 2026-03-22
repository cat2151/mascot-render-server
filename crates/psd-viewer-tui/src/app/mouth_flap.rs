use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use mascot_render_core::{variation_spec_path, DisplayDiff, PsdDocument, PsdEntry, RenderRequest};

use super::support::current_preview_status;
use super::App;
use crate::display_diff_state::{
    find_named_exclusive_pair, resolve_layer_rows, toggle_layer_override,
};

const MOUTH_FLAP_DURATION: Duration = Duration::from_secs(5);
const MOUTH_FLAP_INTERVAL: Duration = Duration::from_millis(250);
const FIRST_FRAME_NAME: &str = "ほあー";
const SECOND_FRAME_NAME: &str = "むふ";

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
    label: &'static str,
    preview_png_path: PathBuf,
    variation_spec_path: Option<PathBuf>,
}

impl App {
    pub(crate) fn start_mouth_flap_preview(&mut self) -> bool {
        let outcome = self.start_mouth_flap_preview_inner();
        match outcome {
            Ok(animation) => {
                let frame = animation.current_frame();
                self.clear_preview_animations();
                self.current_preview_png_path = Some(frame.preview_png_path.clone());
                self.current_variation_spec_path = frame.variation_spec_path.clone();
                self.status = format!(
                    "Mouth flap preview: {} / {} (5s, 250ms)",
                    FIRST_FRAME_NAME, SECOND_FRAME_NAME
                );
                self.mouth_flap = Some(animation);
                true
            }
            Err(error) => {
                self.status = format!("Mouth flap preview unavailable: {error}");
                eprintln!("{error:#}");
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

    fn start_mouth_flap_preview_inner(&self) -> Result<MouthFlapAnimation, String> {
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
            .unwrap_or_else(DisplayDiff::new);
        let (first_row_index, second_row_index) =
            find_named_exclusive_pair(document, FIRST_FRAME_NAME, SECOND_FRAME_NAME).ok_or_else(
                || {
                    format!(
                        "selected PSD does not contain sibling '*{}' and '*{}' layers",
                        FIRST_FRAME_NAME, SECOND_FRAME_NAME
                    )
                },
            )?;
        let frames = [
            self.build_mouth_flap_frame(
                &zip_path,
                &psd_path_in_zip,
                &psd_entry,
                document,
                &base_variation,
                first_row_index,
                FIRST_FRAME_NAME,
            )?,
            self.build_mouth_flap_frame(
                &zip_path,
                &psd_path_in_zip,
                &psd_entry,
                document,
                &base_variation,
                second_row_index,
                SECOND_FRAME_NAME,
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
        zip_path: &Path,
        psd_path_in_zip: &Path,
        psd_entry: &PsdEntry,
        document: &PsdDocument,
        base_variation: &DisplayDiff,
        row_index: usize,
        label: &'static str,
    ) -> Result<MouthFlapFrame, String> {
        let variation = ensure_named_row_visible(base_variation, document, row_index, label)?;
        let (preview_png_path, variation_spec_path) =
            self.render_preview_for_spec(zip_path, psd_path_in_zip, psd_entry, &variation)?;

        Ok(MouthFlapFrame {
            label,
            preview_png_path,
            variation_spec_path,
        })
    }

    fn render_preview_for_spec(
        &self,
        zip_path: &Path,
        psd_path_in_zip: &Path,
        psd_entry: &PsdEntry,
        display_diff: &DisplayDiff,
    ) -> Result<(PathBuf, Option<PathBuf>), String> {
        if display_diff.is_default() {
            let preview_png_path = psd_entry
                .rendered_png_path
                .clone()
                .ok_or_else(|| format!("default PNG is missing for {}", psd_entry.file_name))?;
            return Ok((preview_png_path, None));
        }

        let rendered = self
            .core
            .render_png(RenderRequest {
                zip_path: zip_path.to_path_buf(),
                psd_path_in_zip: psd_path_in_zip.to_path_buf(),
                display_diff: display_diff.clone(),
            })
            .map_err(|error| error.to_string())?;

        Ok((
            rendered.output_path.clone(),
            Some(variation_spec_path(&rendered.output_path)),
        ))
    }
}

fn ensure_named_row_visible(
    base_variation: &DisplayDiff,
    document: &PsdDocument,
    row_index: usize,
    label: &'static str,
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
        Self {
            base_preview_png_path: None,
            base_variation_spec_path: None,
            frames: [
                MouthFlapFrame {
                    label: FIRST_FRAME_NAME,
                    preview_png_path: frames[0].clone(),
                    variation_spec_path: None,
                },
                MouthFlapFrame {
                    label: SECOND_FRAME_NAME,
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

    pub(crate) fn current_frame_label_for_test(&self) -> &'static str {
        self.current_frame().label
    }

    pub(crate) fn is_finished_for_test(&self, now: Instant) -> bool {
        self.is_finished(now)
    }
}

