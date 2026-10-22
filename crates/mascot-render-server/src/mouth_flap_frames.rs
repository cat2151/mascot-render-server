use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{Core, DisplayDiff, PsdDocument, RenderRequest};

use crate::eye_blink::build_closed_eye_display_diff_with_document;

/// The closed-eye frames keep the current mouth shape, so a blink neither
/// pauses the mouth flap nor is suppressed by it.
#[derive(Debug, Clone)]
pub(crate) struct MouthFlapFrames<T> {
    pub(crate) open: T,
    pub(crate) closed: T,
    pub(crate) blink_open: Option<T>,
    pub(crate) blink_closed: Option<T>,
}

impl<T> MouthFlapFrames<T> {
    pub(crate) fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> MouthFlapFrames<U> {
        MouthFlapFrames {
            open: f(&self.open),
            closed: f(&self.closed),
            blink_open: self.blink_open.as_ref().map(&mut f),
            blink_closed: self.blink_closed.as_ref().map(&mut f),
        }
    }

    pub(crate) fn try_map<U>(
        &self,
        mut f: impl FnMut(&T) -> Result<U>,
    ) -> Result<MouthFlapFrames<U>> {
        Ok(MouthFlapFrames {
            open: f(&self.open)?,
            closed: f(&self.closed)?,
            blink_open: self.blink_open.as_ref().map(&mut f).transpose()?,
            blink_closed: self.blink_closed.as_ref().map(&mut f).transpose()?,
        })
    }

    pub(crate) fn labeled(&self) -> [(&'static str, Option<&T>); 4] {
        [
            ("mouth-open", Some(&self.open)),
            ("mouth-closed", Some(&self.closed)),
            ("blink-mouth-open", self.blink_open.as_ref()),
            ("blink-mouth-closed", self.blink_closed.as_ref()),
        ]
    }
}

pub(crate) fn select_frame<'a, T>(
    open: &'a T,
    blink: Option<&'a T>,
    mouth_flap: Option<&'a MouthFlapFrames<T>>,
    mouth_open: Option<bool>,
    blink_closed: bool,
) -> &'a T {
    let frame = match (mouth_flap.zip(mouth_open), blink_closed) {
        (Some((frames, true)), true) => frames.blink_open.as_ref().or(Some(&frames.open)),
        (Some((frames, false)), true) => frames.blink_closed.as_ref().or(Some(&frames.closed)),
        (Some((frames, true)), false) => Some(&frames.open),
        (Some((frames, false)), false) => Some(&frames.closed),
        (None, true) => blink,
        (None, false) => None,
    };
    frame.unwrap_or(open)
}

/// Renders the closed-eye variants of the mouth frames; `None` when the PSD
/// has no eye blink target.
pub(crate) fn render_closed_eye_mouth_frames(
    core: &Core,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    document: &PsdDocument,
    open: &DisplayDiff,
    closed: &DisplayDiff,
) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    let render = |frame: &str, mouth: &DisplayDiff| -> Result<Option<PathBuf>> {
        let Some(display_diff) = build_closed_eye_display_diff_with_document(
            zip_path,
            psd_path_in_zip,
            document,
            mouth,
        )?
        else {
            return Ok(None);
        };
        let rendered = core
            .render_png(RenderRequest {
                zip_path: zip_path.to_path_buf(),
                psd_path_in_zip: psd_path_in_zip.to_path_buf(),
                display_diff,
            })
            .with_context(|| {
                format!(
                    "failed to render closed-eye mouth {frame} PNG {} :: {}",
                    zip_path.display(),
                    psd_path_in_zip.display()
                )
            })?;
        Ok(Some(rendered.output_path))
    };
    Ok((render("open", open)?, render("closed", closed)?))
}
