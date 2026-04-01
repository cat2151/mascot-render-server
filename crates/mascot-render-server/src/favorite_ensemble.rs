use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{
    load_mascot_image, local_data_root, mascot_window_size, Core, DisplayDiff,
    LayerVisibilityOverride, MascotImageData, RenderRequest, DISPLAY_DIFF_VERSION,
};
use mascot_render_server::alpha_bounds_from_mask;
use serde::{Deserialize, Serialize};

use crate::eye_blink::build_closed_eye_display_diff_with_document;

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "favorites.toml";
const FAVORITE_ENSEMBLE_CONTENT_BOUNDS_ALPHA_THRESHOLD: u8 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct FavoriteEnsembleEntry {
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
pub(crate) struct FavoriteEnsembleLayoutEntry {
    pub(crate) size: [f32; 2],
    pub(crate) content_x_bounds: [f32; 2],
    pub(crate) position: Option<[f32; 2]>,
}

#[derive(Debug)]
pub(crate) struct FavoriteEnsembleMember {
    pub(crate) image: MascotImageData,
    pub(crate) closed_image: Option<MascotImageData>,
    pub(crate) base_size: [f32; 2],
    pub(crate) canvas_position: [f32; 2],
}

#[derive(Debug)]
pub(crate) struct FavoriteEnsemble {
    pub(crate) members: Vec<FavoriteEnsembleMember>,
    pub(crate) canvas_size: [f32; 2],
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteEnsembleEntry>,
}

struct RenderedFavorite {
    entry: FavoriteEnsembleEntry,
    image: MascotImageData,
    closed_image: Option<MascotImageData>,
    base_size: [f32; 2],
}

pub(crate) fn favorites_path() -> PathBuf {
    local_data_root()
        .join(FAVORITES_DIR)
        .join(FAVORITES_FILE_NAME)
}

pub(crate) fn load_favorite_ensemble(core: &Core) -> Result<Option<FavoriteEnsemble>> {
    let favorites_path = favorites_path();
    let mut favorites = load_favorites(&favorites_path)?;
    if favorites.is_empty() {
        return Ok(None);
    }

    let mut rendered = favorites
        .drain(..)
        .map(|entry| render_favorite(core, entry))
        .collect::<Result<Vec<_>>>()?;
    if rendered.is_empty() {
        return Ok(None);
    }

    let mut layout_entries = rendered
        .iter()
        .map(layout_entry_from_rendered)
        .collect::<Vec<_>>();
    let updated_indices = fill_missing_positions(&mut layout_entries);
    for (favorite, layout_entry) in rendered.iter_mut().zip(layout_entries) {
        favorite.entry.favorite_ensemble_position = layout_entry.position;
    }
    if !updated_indices.is_empty() {
        patch_favorite_ensemble_positions(
            &favorites_path,
            &updated_indices
                .into_iter()
                .map(|index| rendered[index].entry.clone())
                .collect::<Vec<_>>(),
        )?;
    }

    Ok(Some(build_favorite_ensemble(rendered)))
}

pub(crate) fn pack_positions_from_right(
    layout_entries: &[FavoriteEnsembleLayoutEntry],
) -> Vec<[f32; 2]> {
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

pub(crate) fn fill_missing_positions(
    layout_entries: &mut [FavoriteEnsembleLayoutEntry],
) -> Vec<usize> {
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
    layout_entries: &[FavoriteEnsembleLayoutEntry],
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

fn layout_entry_from_rendered(favorite: &RenderedFavorite) -> FavoriteEnsembleLayoutEntry {
    FavoriteEnsembleLayoutEntry {
        size: favorite.base_size,
        content_x_bounds: scaled_content_x_bounds(
            &favorite.entry,
            &favorite.image,
            favorite.base_size,
        ),
        position: favorite.entry.favorite_ensemble_position,
    }
}

fn visible_width(entry: &FavoriteEnsembleLayoutEntry) -> f32 {
    (entry.content_x_bounds[1] - entry.content_x_bounds[0]).max(0.0)
}

pub(crate) fn scaled_content_x_bounds(
    entry: &FavoriteEnsembleEntry,
    image: &MascotImageData,
    base_size: [f32; 2],
) -> [f32; 2] {
    let alpha_mask = alpha_mask_from_image(image);
    let Some(bounds) = alpha_bounds_from_mask(
        [image.width, image.height],
        &alpha_mask,
        FAVORITE_ENSEMBLE_CONTENT_BOUNDS_ALPHA_THRESHOLD,
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
            "favorite ensemble could not detect visible bounds for {} :: {} ({reason}); using full image width",
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

fn render_favorite(core: &Core, entry: FavoriteEnsembleEntry) -> Result<RenderedFavorite> {
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
                "failed to render favorite ensemble image {} :: {}",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })?;
    let image = load_mascot_image(&rendered.output_path).with_context(|| {
        format!(
            "failed to load favorite ensemble PNG {} :: {} from {}",
            entry.zip_path.display(),
            entry.psd_path_in_zip.display(),
            rendered.output_path.display()
        )
    })?;
    let document = core
        .inspect_psd(&entry.zip_path, &entry.psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect favorite ensemble PSD {} :: {} for eye blink",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })?;
    Ok(RenderedFavorite {
        base_size: mascot_window_size(image.width, image.height, entry.mascot_scale),
        closed_image: build_closed_eye_display_diff_with_document(
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
                    "failed to render favorite ensemble closed-eye PNG {} :: {}",
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
                    "failed to load favorite ensemble closed-eye PNG {} :: {} from {}",
                    entry.zip_path.display(),
                    entry.psd_path_in_zip.display(),
                    rendered_closed.output_path.display()
                )
            })
        })
        .transpose()?,
        entry,
        image,
    })
}

fn build_favorite_ensemble(rendered: Vec<RenderedFavorite>) -> FavoriteEnsemble {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for favorite in &rendered {
        let [x, y] = favorite
            .entry
            .favorite_ensemble_position
            .unwrap_or([0.0, 0.0]);
        let [width, height] = favorite.base_size;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return FavoriteEnsemble {
            members: Vec::new(),
            canvas_size: [1.0, 1.0],
        };
    }

    FavoriteEnsemble {
        canvas_size: [(max_x - min_x).max(1.0), (max_y - min_y).max(1.0)],
        members: rendered
            .into_iter()
            .map(|favorite| {
                let [x, y] = favorite
                    .entry
                    .favorite_ensemble_position
                    .unwrap_or([0.0, 0.0]);
                FavoriteEnsembleMember {
                    canvas_position: [x - min_x, y - min_y],
                    base_size: favorite.base_size,
                    closed_image: favorite.closed_image,
                    image: favorite.image,
                }
            })
            .collect(),
    }
}

fn load_favorites(path: &Path) -> Result<Vec<FavoriteEnsembleEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read favorite ensemble entries {}",
            path.display()
        )
    })?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(sanitize_favorites(file.favorites)),
        Err(error) => {
            eprintln!(
                "favorite ensemble ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(Vec::new())
        }
    }
}

fn patch_favorite_ensemble_positions(path: &Path, updates: &[FavoriteEnsembleEntry]) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read favorite ensemble entries {}",
            path.display()
        )
    })?;
    let patched = patch_favorite_ensemble_positions_toml(&raw, updates)?;
    fs::write(path, patched).with_context(|| {
        format!(
            "failed to write favorite ensemble entries {}",
            path.display()
        )
    })
}

pub(crate) fn patch_favorite_ensemble_positions_toml(
    raw: &str,
    updates: &[FavoriteEnsembleEntry],
) -> Result<String> {
    let mut value = toml::from_str::<toml::Value>(raw)
        .context("failed to parse favorites TOML while patching ensemble positions")?;
    let favorites = value
        .get_mut("favorites")
        .and_then(toml::Value::as_array_mut)
        .context("favorites should remain an array while patching ensemble positions")?;

    for update in updates {
        let Some(position) = update.favorite_ensemble_position else {
            continue;
        };
        let Some(entry) = favorites
            .iter_mut()
            .find(|entry| favorite_entry_matches_update(entry, update))
        else {
            continue;
        };
        // Only backfill entries missing favorite_ensemble_position, preserving
        // user-adjusted coordinates.
        if entry
            .get("favorite_ensemble_position")
            .and_then(toml::Value::as_array)
            .is_some()
        {
            continue;
        }

        let Some(table) = entry.as_table_mut() else {
            continue;
        };
        table.insert(
            "favorite_ensemble_position".to_string(),
            toml::Value::Array(vec![position[0].into(), position[1].into()]),
        );
    }

    toml::to_string_pretty(&value).context("failed to serialize patched favorites TOML")
}

fn favorite_entry_matches_update(value: &toml::Value, update: &FavoriteEnsembleEntry) -> bool {
    let Some(table) = value.as_table() else {
        return false;
    };
    let zip_path = table
        .get("zip_path")
        .and_then(toml::Value::as_str)
        .map(Path::new);
    let psd_path_in_zip = table
        .get("psd_path_in_zip")
        .and_then(toml::Value::as_str)
        .map(Path::new);
    zip_path == Some(update.zip_path.as_path())
        && psd_path_in_zip == Some(update.psd_path_in_zip.as_path())
        && table_visibility_overrides(table.get("visibility_overrides"))
            == update
                .visibility_overrides
                .iter()
                .map(|layer| (layer.layer_index, layer.visible))
                .collect::<Vec<_>>()
}

fn table_visibility_overrides(value: Option<&toml::Value>) -> Vec<(usize, bool)> {
    value
        .and_then(toml::Value::as_array)
        .map(|layers| {
            layers
                .iter()
                .filter_map(|layer| {
                    let table = layer.as_table()?;
                    let layer_index_value =
                        table.get("layer_index").and_then(toml::Value::as_integer)?;
                    let layer_index = match layer_index_value.try_into() {
                        Ok(layer_index) => layer_index,
                        Err(_) => {
                            eprintln!(
                                "favorite ensemble ignored invalid layer_index {} while matching visibility_overrides",
                                layer_index_value
                            );
                            return None;
                        }
                    };
                    let visible = table.get("visible").and_then(toml::Value::as_bool)?;
                    Some((layer_index, visible))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn sanitize_favorites(favorites: Vec<FavoriteEnsembleEntry>) -> Vec<FavoriteEnsembleEntry> {
    let mut sanitized = Vec::new();
    for mut favorite in favorites {
        if favorite.zip_path.as_os_str().is_empty()
            || favorite.psd_path_in_zip.as_os_str().is_empty()
        {
            continue;
        }
        if favorite.psd_file_name.is_empty() {
            favorite.psd_file_name = favorite
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| favorite.psd_path_in_zip.display().to_string());
        }
        favorite.mascot_scale = sanitize_scale(favorite.mascot_scale);
        favorite.favorite_ensemble_position =
            sanitize_position(favorite.favorite_ensemble_position);
        if let Some(index) = sanitized.iter().position(|saved: &FavoriteEnsembleEntry| {
            favorite_identity(saved) == favorite_identity(&favorite)
        }) {
            sanitized[index] = favorite;
        } else {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn favorite_identity(favorite: &FavoriteEnsembleEntry) -> (&Path, &Path, Vec<(usize, bool)>) {
    (
        favorite.zip_path.as_path(),
        favorite.psd_path_in_zip.as_path(),
        favorite
            .visibility_overrides
            .iter()
            .map(|layer| (layer.layer_index, layer.visible))
            .collect(),
    )
}

fn sanitize_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn sanitize_position(position: Option<[f32; 2]>) -> Option<[f32; 2]> {
    position.filter(|[x, y]| x.is_finite() && y.is_finite())
}
