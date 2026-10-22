use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use mascot_render_core::{
    auto_generate_mouth_flap_target, build_mouth_flap_display_diffs, load_mascot_image, Core,
    DisplayDiff, MascotImageData, PsdDocument, RenderRequest,
};

use crate::mouth_flap_frames::{render_closed_eye_mouth_frames, MouthFlapFrames};

pub(super) fn render_mouth_flap_images_with_document(
    core: &Core,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    document: &PsdDocument,
    base_variation: &DisplayDiff,
) -> Result<Option<MouthFlapFrames<MascotImageData>>> {
    let psd_file_name = psd_file_name(psd_path_in_zip)?;
    let target = match auto_generate_mouth_flap_target(document, base_variation) {
        Ok(target) => target,
        Err(error) => {
            eprintln!(
                "ensemble mouth flap auto-generation skipped: zip_path={} psd_path_in_zip={} reason={}",
                zip_path.display(),
                psd_path_in_zip.display(),
                error
            );
            return Ok(None);
        }
    };
    let display_diffs = build_mouth_flap_display_diffs(document, base_variation, &target)
        .map_err(|error| anyhow!(error))
        .with_context(|| {
            format!(
                "failed to build ensemble mouth flap variations for '{}'",
                psd_file_name
            )
        })?;

    let (blink_open, blink_closed) = render_closed_eye_mouth_frames(
        core,
        zip_path,
        psd_path_in_zip,
        document,
        &display_diffs.open,
        &display_diffs.closed,
    )?;
    let load_blink_image = |png_path: PathBuf| {
        load_mascot_image(&png_path).with_context(|| {
            format!(
                "failed to load ensemble closed-eye mouth PNG {} :: {} from {}",
                zip_path.display(),
                psd_path_in_zip.display(),
                png_path.display()
            )
        })
    };

    Ok(Some(MouthFlapFrames {
        open: render_mouth_flap_image(core, zip_path, psd_path_in_zip, "open", display_diffs.open)?,
        closed: render_mouth_flap_image(
            core,
            zip_path,
            psd_path_in_zip,
            "closed",
            display_diffs.closed,
        )?,
        blink_open: blink_open.map(load_blink_image).transpose()?,
        blink_closed: blink_closed.map(load_blink_image).transpose()?,
    }))
}

fn render_mouth_flap_image(
    core: &Core,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    frame: &'static str,
    display_diff: DisplayDiff,
) -> Result<MascotImageData> {
    let rendered = core
        .render_png(RenderRequest {
            zip_path: zip_path.to_path_buf(),
            psd_path_in_zip: psd_path_in_zip.to_path_buf(),
            display_diff,
        })
        .with_context(|| {
            format!(
                "failed to render ensemble mouth flap {frame} PNG for '{}'",
                psd_file_name(psd_path_in_zip)
                    .unwrap_or_else(|_| psd_path_in_zip.display().to_string())
            )
        })?;

    load_mascot_image(&rendered.output_path).with_context(|| {
        format!(
            "failed to load ensemble mouth flap {frame} PNG {} :: {} from {}",
            zip_path.display(),
            psd_path_in_zip.display(),
            rendered.output_path.display()
        )
    })
}

fn psd_file_name(psd_path_in_zip: &Path) -> Result<String> {
    psd_path_in_zip
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("invalid PSD file name in '{}'", psd_path_in_zip.display()))
}
