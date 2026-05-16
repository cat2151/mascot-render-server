use std::path::PathBuf;

use anyhow::{Context, Result};
use mascot_render_core::{
    load_mascot_image, local_data_root, mascot_window_size, Core, DisplayDiff,
    LayerVisibilityOverride, MascotEnsembleMode, MascotImageData, RenderRequest,
    DISPLAY_DIFF_VERSION,
};
use mascot_render_server::alpha_bounds_from_mask;
use serde::{Deserialize, Serialize};

use crate::eye_blink::build_closed_eye_display_diff_with_document;
use mouth_flap::render_mouth_flap_images_with_document;
#[path = "ensemble/mouth_flap.rs"]
mod mouth_flap;
#[path = "ensemble/persistence.rs"]
mod persistence;
#[path = "ensemble/sanitize.rs"]
mod sanitize;
#[cfg(test)]
pub(crate) use persistence::patch_ensemble_positions_toml;
use persistence::{load_ensemble_entries, patch_ensemble_positions, write_ensemble_entries};
#[cfg(test)]
pub(crate) use sanitize::sanitize_ensemble_entries_for_test;

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "favorites.toml";
const VPT_ENSEMBLE_DIR: &str = "vpt-ensemble";
const VPT_ENSEMBLE_FILE_NAME: &str = "ensemble.toml";
const ENSEMBLE_CONTENT_BOUNDS_ALPHA_THRESHOLD: u8 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct EnsembleEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) character_name: Option<String>,
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    #[serde(default)]
    pub(crate) psd_file_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) visibility_overrides: Vec<LayerVisibilityOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mascot_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) favorite_ensemble_position: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EnsembleLayoutEntry {
    pub(crate) size: [f32; 2],
    pub(crate) content_x_bounds: [f32; 2],
    pub(crate) position: Option<[f32; 2]>,
}

#[derive(Debug)]
pub(crate) struct EnsembleMember {
    pub(crate) character_name: Option<String>,
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) image: MascotImageData,
    pub(crate) closed_image: Option<MascotImageData>,
    pub(crate) mouth_open_image: Option<MascotImageData>,
    pub(crate) mouth_closed_image: Option<MascotImageData>,
    pub(crate) base_size: [f32; 2],
    pub(crate) canvas_position: [f32; 2],
}

#[derive(Debug)]
pub(crate) struct Ensemble {
    pub(crate) members: Vec<EnsembleMember>,
    pub(crate) canvas_size: [f32; 2],
}

struct RenderedEnsembleEntry {
    entry: EnsembleEntry,
    image: MascotImageData,
    closed_image: Option<MascotImageData>,
    mouth_open_image: Option<MascotImageData>,
    mouth_closed_image: Option<MascotImageData>,
    base_size: [f32; 2],
}

pub(crate) fn favorites_path() -> PathBuf {
    local_data_root()
        .join(FAVORITES_DIR)
        .join(FAVORITES_FILE_NAME)
}

pub(crate) fn vpt_ensemble_path() -> PathBuf {
    local_data_root()
        .join(VPT_ENSEMBLE_DIR)
        .join(VPT_ENSEMBLE_FILE_NAME)
}

pub(crate) fn active_ensemble_path(mode: MascotEnsembleMode) -> Option<PathBuf> {
    match mode {
        MascotEnsembleMode::SingleCharacter => None,
        MascotEnsembleMode::Favorite => Some(favorites_path()),
        MascotEnsembleMode::Vpt => Some(vpt_ensemble_path()),
    }
}

pub(crate) fn load_active_ensemble(
    core: &Core,
    mode: MascotEnsembleMode,
) -> Result<Option<Ensemble>> {
    let Some(path) = active_ensemble_path(mode) else {
        return Ok(None);
    };
    load_ensemble_from_path(core, path)
}

pub(crate) fn save_vpt_ensemble(entries: &[EnsembleEntry]) -> Result<()> {
    write_ensemble_entries(&vpt_ensemble_path(), entries)
}

pub(crate) fn load_vpt_ensemble_entries() -> Result<Vec<EnsembleEntry>> {
    load_ensemble_entries(&vpt_ensemble_path())
}

pub(crate) fn load_ensemble_from_path(
    core: &Core,
    ensemble_path: PathBuf,
) -> Result<Option<Ensemble>> {
    let mut entries = load_ensemble_entries(&ensemble_path)?;
    if entries.is_empty() {
        return Ok(None);
    }

    let mut rendered = entries
        .drain(..)
        .map(|entry| render_ensemble_entry(core, entry))
        .collect::<Result<Vec<_>>>()?;
    if rendered.is_empty() {
        return Ok(None);
    }

    let mut layout_entries = rendered
        .iter()
        .map(layout_entry_from_rendered)
        .collect::<Vec<_>>();
    let updated_indices = fill_missing_positions(&mut layout_entries);
    for (rendered_entry, layout_entry) in rendered.iter_mut().zip(layout_entries) {
        rendered_entry.entry.favorite_ensemble_position = layout_entry.position;
    }
    if !updated_indices.is_empty() {
        patch_ensemble_positions(
            &ensemble_path,
            &updated_indices
                .into_iter()
                .map(|index| rendered[index].entry.clone())
                .collect::<Vec<_>>(),
        )?;
    }

    Ok(Some(build_ensemble(rendered)))
}

pub(crate) fn pack_positions_from_right(layout_entries: &[EnsembleLayoutEntry]) -> Vec<[f32; 2]> {
    let total_visible_width = layout_entries.iter().map(visible_width).sum::<f32>();
    let max_height = layout_entries
        .iter()
        .map(|entry| entry.size[1])
        .fold(0.0, f32::max);
    let mut next_visible_right_edge = total_visible_width;
    let mut positions = Vec::with_capacity(layout_entries.len());
    for entry in layout_entries {
        positions.push([
            next_visible_right_edge - entry.content_x_bounds[1],
            max_height - entry.size[1],
        ]);
        next_visible_right_edge -= visible_width(entry);
    }
    positions
}

pub(crate) fn fill_missing_positions(layout_entries: &mut [EnsembleLayoutEntry]) -> Vec<usize> {
    let missing_indices = layout_entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.position.is_none().then_some(index))
        .collect::<Vec<_>>();
    if missing_indices.is_empty() {
        return Vec::new();
    }

    let mut existing_right_edge = None::<f32>;
    let mut existing_bottom = None::<f32>;
    let mut max_height = 0.0_f32;
    for entry in layout_entries.iter() {
        max_height = max_height.max(entry.size[1]);
        if let Some([x, y]) = entry.position {
            let visible_left = x + entry.content_x_bounds[0];
            existing_right_edge =
                Some(existing_right_edge.map_or(visible_left, |current| current.min(visible_left)));
            let bottom = y + entry.size[1];
            existing_bottom = Some(existing_bottom.map_or(bottom, |current| current.max(bottom)));
        }
    }
    let bottom = existing_bottom.unwrap_or(max_height);

    let missing_entries = missing_indices
        .iter()
        .map(|&index| layout_entries[index])
        .collect::<Vec<_>>();
    let positions = if let Some(right_edge) = existing_right_edge {
        pack_positions_with_right_edge(&missing_entries, right_edge, bottom)
    } else {
        pack_positions_from_right(&missing_entries)
    };

    for (index, position) in missing_indices.iter().copied().zip(positions) {
        layout_entries[index].position = Some(position);
    }
    missing_indices
}

fn pack_positions_with_right_edge(
    layout_entries: &[EnsembleLayoutEntry],
    right_edge: f32,
    bottom: f32,
) -> Vec<[f32; 2]> {
    let mut next_visible_right_edge = right_edge;
    let mut positions = Vec::with_capacity(layout_entries.len());
    for entry in layout_entries {
        positions.push([
            next_visible_right_edge - entry.content_x_bounds[1],
            bottom - entry.size[1],
        ]);
        next_visible_right_edge -= visible_width(entry);
    }
    positions
}

fn layout_entry_from_rendered(rendered_entry: &RenderedEnsembleEntry) -> EnsembleLayoutEntry {
    EnsembleLayoutEntry {
        size: rendered_entry.base_size,
        content_x_bounds: scaled_content_x_bounds(
            &rendered_entry.entry,
            &rendered_entry.image,
            rendered_entry.base_size,
        ),
        position: rendered_entry.entry.favorite_ensemble_position,
    }
}

fn visible_width(entry: &EnsembleLayoutEntry) -> f32 {
    (entry.content_x_bounds[1] - entry.content_x_bounds[0]).max(0.0)
}

pub(crate) fn scaled_content_x_bounds(
    entry: &EnsembleEntry,
    image: &MascotImageData,
    base_size: [f32; 2],
) -> [f32; 2] {
    let alpha_mask = alpha_mask_from_image(image);
    let Some(bounds) = alpha_bounds_from_mask(
        [image.width, image.height],
        &alpha_mask,
        ENSEMBLE_CONTENT_BOUNDS_ALPHA_THRESHOLD,
    ) else {
        let reason = if alpha_mask.len() != image.width as usize * image.height as usize {
            format!(
                "invalid alpha mask length {} for image size {}x{}",
                alpha_mask.len(),
                image.width,
                image.height
            )
        } else {
            "image is fully transparent".to_string()
        };
        eprintln!(
            "ensemble could not detect visible bounds for {} :: {} ({reason}); using full image width",
            entry.zip_path.display(),
            entry.psd_path_in_zip.display()
        );
        return [0.0, base_size[0]];
    };
    let scale = base_size[0] / image.width as f32;
    let left = (bounds.min_x as f32 * scale).clamp(0.0, base_size[0]);
    let raw_right = (bounds.max_x as f32 * scale).clamp(0.0, base_size[0]);
    let right = raw_right.max(left);
    [left, right]
}

fn alpha_mask_from_image(image: &MascotImageData) -> Vec<u8> {
    image
        .rgba
        .chunks_exact(4)
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>()
}

fn render_ensemble_entry(core: &Core, entry: EnsembleEntry) -> Result<RenderedEnsembleEntry> {
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
        mouth_open_image: mouth_flap_images.as_ref().map(|images| images.open.clone()),
        mouth_closed_image: mouth_flap_images.map(|images| images.closed),
        entry,
        image,
    })
}

fn build_ensemble(rendered: Vec<RenderedEnsembleEntry>) -> Ensemble {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for rendered_entry in &rendered {
        let [x, y] = rendered_entry
            .entry
            .favorite_ensemble_position
            .unwrap_or([0.0, 0.0]);
        let [width, height] = rendered_entry.base_size;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return Ensemble {
            members: Vec::new(),
            canvas_size: [1.0, 1.0],
        };
    }

    Ensemble {
        canvas_size: [(max_x - min_x).max(1.0), (max_y - min_y).max(1.0)],
        members: rendered
            .into_iter()
            .map(|rendered_entry| {
                let [x, y] = rendered_entry
                    .entry
                    .favorite_ensemble_position
                    .unwrap_or([0.0, 0.0]);
                EnsembleMember {
                    character_name: rendered_entry.entry.character_name,
                    zip_path: rendered_entry.entry.zip_path,
                    psd_path_in_zip: rendered_entry.entry.psd_path_in_zip,
                    canvas_position: [x - min_x, y - min_y],
                    base_size: rendered_entry.base_size,
                    closed_image: rendered_entry.closed_image,
                    mouth_open_image: rendered_entry.mouth_open_image,
                    mouth_closed_image: rendered_entry.mouth_closed_image,
                    image: rendered_entry.image,
                }
            })
            .collect(),
    }
}
