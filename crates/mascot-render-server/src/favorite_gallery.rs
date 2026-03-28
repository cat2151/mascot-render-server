use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{
    load_mascot_image, local_data_root, mascot_window_size, Core, DisplayDiff,
    LayerVisibilityOverride, MascotImageData, RenderRequest, DISPLAY_DIFF_VERSION,
};
use serde::{Deserialize, Serialize};

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "favorites.toml";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct FavoriteGalleryEntry {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    #[serde(default)]
    pub(crate) psd_file_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) visibility_overrides: Vec<LayerVisibilityOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mascot_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) favorite_gallery_position: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FavoriteGalleryLayoutEntry {
    pub(crate) size: [f32; 2],
    pub(crate) position: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteGalleryEntry>,
}

struct RenderedFavorite {
    entry: FavoriteGalleryEntry,
    image: MascotImageData,
    base_size: [f32; 2],
}

pub(crate) fn favorites_path() -> PathBuf {
    local_data_root()
        .join(FAVORITES_DIR)
        .join(FAVORITES_FILE_NAME)
}

pub(crate) fn load_gallery_image(core: &Core) -> Result<Option<MascotImageData>> {
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
        .map(|favorite| FavoriteGalleryLayoutEntry {
            size: favorite.base_size,
            position: favorite.entry.favorite_gallery_position,
        })
        .collect::<Vec<_>>();
    let updated_indices = fill_missing_positions(&mut layout_entries);
    for (favorite, layout_entry) in rendered.iter_mut().zip(layout_entries) {
        favorite.entry.favorite_gallery_position = layout_entry.position;
    }
    if !updated_indices.is_empty() {
        patch_favorite_gallery_positions(
            &favorites_path,
            &updated_indices
                .into_iter()
                .map(|index| rendered[index].entry.clone())
                .collect::<Vec<_>>(),
        )?;
    }

    Ok(Some(compose_gallery_image(&rendered)))
}

pub(crate) fn pack_positions_from_right(sizes: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let total_width = sizes.iter().map(|[width, _]| *width).sum::<f32>();
    let max_height = sizes.iter().map(|[_, height]| *height).fold(0.0, f32::max);
    let mut next_right_edge = total_width;
    let mut positions = Vec::with_capacity(sizes.len());
    for [width, height] in sizes {
        next_right_edge -= *width;
        positions.push([next_right_edge, max_height - *height]);
    }
    positions
}

pub(crate) fn fill_missing_positions(
    layout_entries: &mut [FavoriteGalleryLayoutEntry],
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
            existing_right_edge = Some(existing_right_edge.map_or(x, |current| current.min(x)));
            let bottom = y + entry.size[1];
            existing_bottom = Some(existing_bottom.map_or(bottom, |current| current.max(bottom)));
        }
    }
    let bottom = existing_bottom.unwrap_or(max_height);

    let missing_sizes = missing_indices
        .iter()
        .map(|&index| layout_entries[index].size)
        .collect::<Vec<_>>();
    let positions = if let Some(right_edge) = existing_right_edge {
        pack_positions_with_right_edge(&missing_sizes, right_edge, bottom)
    } else {
        pack_positions_from_right(&missing_sizes)
    };

    for (index, position) in missing_indices.iter().copied().zip(positions) {
        layout_entries[index].position = Some(position);
    }
    missing_indices
}

fn pack_positions_with_right_edge(
    sizes: &[[f32; 2]],
    right_edge: f32,
    bottom: f32,
) -> Vec<[f32; 2]> {
    let mut next_right_edge = right_edge;
    let mut positions = Vec::with_capacity(sizes.len());
    for [width, height] in sizes {
        next_right_edge -= *width;
        positions.push([next_right_edge, bottom - *height]);
    }
    positions
}

fn render_favorite(core: &Core, entry: FavoriteGalleryEntry) -> Result<RenderedFavorite> {
    let display_diff = DisplayDiff {
        version: DISPLAY_DIFF_VERSION,
        visibility_overrides: entry.visibility_overrides.clone(),
    };
    let rendered = core
        .render_png(RenderRequest {
            zip_path: entry.zip_path.clone(),
            psd_path_in_zip: entry.psd_path_in_zip.clone(),
            display_diff,
        })
        .with_context(|| {
            format!(
                "failed to render favorite gallery image {} :: {}",
                entry.zip_path.display(),
                entry.psd_path_in_zip.display()
            )
        })?;
    let image = load_mascot_image(&rendered.output_path).with_context(|| {
        format!(
            "failed to load favorite gallery PNG {} :: {} from {}",
            entry.zip_path.display(),
            entry.psd_path_in_zip.display(),
            rendered.output_path.display()
        )
    })?;
    Ok(RenderedFavorite {
        base_size: mascot_window_size(image.width, image.height, entry.mascot_scale),
        entry,
        image,
    })
}

fn compose_gallery_image(rendered: &[RenderedFavorite]) -> MascotImageData {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for favorite in rendered {
        let [x, y] = favorite
            .entry
            .favorite_gallery_position
            .unwrap_or([0.0, 0.0]);
        let [width, height] = favorite.base_size;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }

    let canvas_width = ((max_x - min_x).ceil() as u32).max(1);
    let canvas_height = ((max_y - min_y).ceil() as u32).max(1);
    let mut rgba = vec![0; canvas_width as usize * canvas_height as usize * 4];

    for favorite in rendered {
        let [x, y] = favorite
            .entry
            .favorite_gallery_position
            .unwrap_or([0.0, 0.0]);
        let [width, height] = favorite.base_size;
        let dest_x = (x - min_x).round() as i32;
        let dest_y = (y - min_y).round() as i32;
        let dest_width = width.round().max(1.0) as u32;
        let dest_height = height.round().max(1.0) as u32;
        blit_nearest_rgba(
            &favorite.image.rgba,
            [favorite.image.width, favorite.image.height],
            &mut rgba,
            [canvas_width, canvas_height],
            [dest_x, dest_y],
            [dest_width, dest_height],
        );
    }

    MascotImageData {
        path: local_data_root().join("favorite-gallery.png"),
        width: canvas_width,
        height: canvas_height,
        rgba,
    }
}

fn blit_nearest_rgba(
    source_rgba: &[u8],
    source_size: [u32; 2],
    dest_rgba: &mut [u8],
    dest_size: [u32; 2],
    dest_origin: [i32; 2],
    dest_draw_size: [u32; 2],
) {
    let [source_width, source_height] = source_size;
    let [dest_width, dest_height] = dest_size;
    let [dest_x, dest_y] = dest_origin;
    let [draw_width, draw_height] = dest_draw_size;
    if source_width == 0 || source_height == 0 || draw_width == 0 || draw_height == 0 {
        return;
    }

    for draw_y in 0..draw_height {
        let canvas_y = dest_y + draw_y as i32;
        if !(0..dest_height as i32).contains(&canvas_y) {
            continue;
        }
        let source_y = ((draw_y as u64 * source_height as u64) / draw_height as u64)
            .min(source_height.saturating_sub(1) as u64) as u32;
        for draw_x in 0..draw_width {
            let canvas_x = dest_x + draw_x as i32;
            if !(0..dest_width as i32).contains(&canvas_x) {
                continue;
            }
            let source_x = ((draw_x as u64 * source_width as u64) / draw_width as u64)
                .min(source_width.saturating_sub(1) as u64) as u32;
            let source_index = ((source_y * source_width + source_x) * 4) as usize;
            let dest_index = (((canvas_y as u32) * dest_width + canvas_x as u32) * 4) as usize;
            blend_pixel(
                &source_rgba[source_index..source_index + 4],
                &mut dest_rgba[dest_index..dest_index + 4],
            );
        }
    }
}

fn blend_pixel(source: &[u8], dest: &mut [u8]) {
    let source_alpha = source[3] as f32 / 255.0;
    if source_alpha <= f32::EPSILON {
        return;
    }
    let dest_alpha = dest[3] as f32 / 255.0;
    let out_alpha = source_alpha + dest_alpha * (1.0 - source_alpha);
    if out_alpha <= f32::EPSILON {
        dest.fill(0);
        return;
    }

    for channel in 0..3 {
        let source_value = source[channel] as f32 / 255.0;
        let dest_value = dest[channel] as f32 / 255.0;
        let out_value = (source_value * source_alpha
            + dest_value * dest_alpha * (1.0 - source_alpha))
            / out_alpha;
        dest[channel] = (out_value * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dest[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn load_favorites(path: &Path) -> Result<Vec<FavoriteGalleryEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorite gallery entries {}", path.display()))?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(sanitize_favorites(file.favorites)),
        Err(error) => {
            eprintln!(
                "favorite gallery ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(Vec::new())
        }
    }
}

fn patch_favorite_gallery_positions(path: &Path, updates: &[FavoriteGalleryEntry]) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorite gallery entries {}", path.display()))?;
    let patched = patch_favorite_gallery_positions_toml(&raw, updates)?;
    fs::write(path, patched).with_context(|| {
        format!(
            "failed to write favorite gallery entries {}",
            path.display()
        )
    })
}

pub(crate) fn patch_favorite_gallery_positions_toml(
    raw: &str,
    updates: &[FavoriteGalleryEntry],
) -> Result<String> {
    let mut value = toml::from_str::<toml::Value>(raw)
        .context("failed to parse favorites TOML while patching gallery positions")?;
    let favorites = value
        .get_mut("favorites")
        .and_then(toml::Value::as_array_mut)
        .context("favorites should remain an array while patching gallery positions")?;

    for update in updates {
        let Some(position) = update.favorite_gallery_position else {
            continue;
        };
        let Some(entry) = favorites
            .iter_mut()
            .find(|entry| favorite_entry_matches_update(entry, update))
        else {
            continue;
        };
        // Only backfill entries missing favorite_gallery_position, preserving
        // user-adjusted coordinates.
        if entry
            .get("favorite_gallery_position")
            .and_then(toml::Value::as_array)
            .is_some()
        {
            continue;
        }

        let Some(table) = entry.as_table_mut() else {
            continue;
        };
        table.insert(
            "favorite_gallery_position".to_string(),
            toml::Value::Array(vec![position[0].into(), position[1].into()]),
        );
    }

    toml::to_string_pretty(&value).context("failed to serialize patched favorites TOML")
}

fn favorite_entry_matches_update(value: &toml::Value, update: &FavoriteGalleryEntry) -> bool {
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
                                "favorite gallery ignored invalid layer_index {} while matching visibility_overrides",
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

fn sanitize_favorites(favorites: Vec<FavoriteGalleryEntry>) -> Vec<FavoriteGalleryEntry> {
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
        favorite.favorite_gallery_position = sanitize_position(favorite.favorite_gallery_position);
        if let Some(index) = sanitized.iter().position(|saved: &FavoriteGalleryEntry| {
            favorite_identity(saved) == favorite_identity(&favorite)
        }) {
            sanitized[index] = favorite;
        } else {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn favorite_identity(favorite: &FavoriteGalleryEntry) -> (&Path, &Path, Vec<(usize, bool)>) {
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
