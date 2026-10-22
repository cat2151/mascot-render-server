use anyhow::{Context, Result};
use mascot_render_core::{
    load_mascot_image, mascot_window_size, Core, DisplayDiff, MascotImageData, RenderRequest,
    DISPLAY_DIFF_VERSION,
};

use super::mouth_flap::render_mouth_flap_images_with_document;
use super::EnsembleEntry;
use crate::eye_blink::build_closed_eye_display_diff_with_document;
use crate::mouth_flap_frames::MouthFlapFrames;

pub(super) struct RenderedEnsembleEntry {
    pub(super) entry: EnsembleEntry,
    pub(super) image: MascotImageData,
    pub(super) closed_image: Option<MascotImageData>,
    pub(super) mouth_flap_images: Option<MouthFlapFrames<MascotImageData>>,
    pub(super) base_size: [f32; 2],
}

pub(super) fn render_ensemble_entry(
    core: &Core,
    entry: EnsembleEntry,
) -> Result<RenderedEnsembleEntry> {
    let display_diff = DisplayDiff {
        version: DISPLAY_DIFF_VERSION,
        visibility_overrides: entry.visibility_overrides.clone(),
    };
    let rendered = core
        .render_png(RenderRequest {
            zip_path: entry.zip_path.clone(),
            psd_path_in_zip: entry.psd_path_in_zip.clone(),
            display_diff: display_diff.clone(),
        })
        .with_context(|| {
            format!(
                "failed to render ensemble image {} :: {}",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })?;
    let image = load_mascot_image(&rendered.output_path).with_context(|| {
        format!(
            "failed to load ensemble PNG {} :: {} from {}",
            entry.zip_path.display(),
            entry.psd_path_in_zip.display(),
            rendered.output_path.display()
        )
    })?;
    let document = core
        .inspect_psd(&entry.zip_path, &entry.psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect ensemble PSD {} :: {} for auxiliary skins",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })?;
    let closed_image = build_closed_eye_display_diff_with_document(
        &entry.zip_path,
        &entry.psd_path_in_zip,
        &document,
        &display_diff,
    )?
    .map(|closed_display_diff| {
        core.render_png(RenderRequest {
            zip_path: entry.zip_path.clone(),
            psd_path_in_zip: entry.psd_path_in_zip.clone(),
            display_diff: closed_display_diff,
        })
        .with_context(|| {
            format!(
                "failed to render ensemble closed-eye PNG {} :: {}",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })
    })
    .transpose()?
    .filter(|rendered_closed| rendered_closed.output_path != rendered.output_path)
    .map(|rendered_closed| {
        load_mascot_image(&rendered_closed.output_path).with_context(|| {
            format!(
                "failed to load ensemble closed-eye PNG {} :: {} from {}",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display(),
                rendered_closed.output_path.display()
            )
        })
    })
    .transpose()?;
    let mouth_flap_images = render_mouth_flap_images_with_document(
        core,
        &entry.zip_path,
        &entry.psd_path_in_zip,
        &document,
        &display_diff,
    )?;

    Ok(RenderedEnsembleEntry {
        base_size: mascot_window_size(image.width, image.height, entry.mascot_scale),
        closed_image,
        mouth_flap_images,
        entry,
        image,
    })
}
