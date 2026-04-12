use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use image::{ColorType, ImageFormat};
use rawpsd::{LayerInfo, PsdMetadata};

use crate::rgba_cache::write_default_rgba_cache_for_rgba;
use crate::skin_details::write_skin_details_cache_for_rgba;

const COLOR_MODE_GRAYSCALE: u16 = 1;
const COLOR_MODE_RGB: u16 = 3;
const COLOR_MODE_CMYK: u16 = 4;

#[derive(Debug, Clone, Default)]
pub(crate) struct RenderResult {
    pub(crate) output_path: PathBuf,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderSidecars {
    pub(crate) raw_rgba: bool,
}

pub(crate) fn render_png(
    metadata: &PsdMetadata,
    layers: &[LayerInfo],
    effective_visibility: &[bool],
    output_path: &Path,
    sidecars: RenderSidecars,
) -> Result<RenderResult, String> {
    let mut canvas = vec![0u8; (metadata.width * metadata.height * 4) as usize];
    let mut warnings = BTreeSet::new();

    for (index, layer) in layers.iter().enumerate() {
        if layer.group_opener || layer.group_closer {
            continue;
        }

        if !effective_visibility.get(index).copied().unwrap_or(false) {
            continue;
        }

        if layer.blend_mode != "norm" {
            warnings.insert(format!(
                "visible layer '{}' uses unsupported blend mode '{}' and was rendered as normal",
                layer.name.trim(),
                layer.blend_mode
            ));
        }

        if layer.is_clipped {
            warnings.insert(format!(
                "visible layer '{}' is a clipping mask and was rendered without clipping",
                layer.name.trim()
            ));
        }

        composite_layer(&mut canvas, metadata, layer);
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    image::save_buffer_with_format(
        output_path,
        &canvas,
        metadata.width,
        metadata.height,
        ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|error| error.to_string())?;

    write_skin_details_cache_for_rgba(output_path, [metadata.width, metadata.height], &canvas)
        .map_err(|error| error.to_string())?;
    if sidecars.raw_rgba {
        write_default_rgba_cache_for_rgba(output_path, [metadata.width, metadata.height], &canvas)
            .map_err(|error| error.to_string())?;
    }

    Ok(RenderResult {
        output_path: output_path.to_path_buf(),
        warnings: warnings.into_iter().collect(),
    })
}

pub(crate) fn blend_pixel(dst: [u8; 4], src: [u8; 4], opacity: f32) -> [u8; 4] {
    let src_alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    let dst_alpha = dst[3] as f32 / 255.0;
    let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);

    if out_alpha <= f32::EPSILON {
        return [0, 0, 0, 0];
    }

    let src_rgb = [
        src[0] as f32 / 255.0,
        src[1] as f32 / 255.0,
        src[2] as f32 / 255.0,
    ];
    let dst_rgb = [
        dst[0] as f32 / 255.0,
        dst[1] as f32 / 255.0,
        dst[2] as f32 / 255.0,
    ];

    let out_rgb = [
        ((src_rgb[0] * src_alpha) + (dst_rgb[0] * dst_alpha * (1.0 - src_alpha))) / out_alpha,
        ((src_rgb[1] * src_alpha) + (dst_rgb[1] * dst_alpha * (1.0 - src_alpha))) / out_alpha,
        ((src_rgb[2] * src_alpha) + (dst_rgb[2] * dst_alpha * (1.0 - src_alpha))) / out_alpha,
    ];

    [
        float_to_u8(out_rgb[0]),
        float_to_u8(out_rgb[1]),
        float_to_u8(out_rgb[2]),
        float_to_u8(out_alpha),
    ]
}

fn composite_layer(canvas: &mut [u8], metadata: &PsdMetadata, layer: &LayerInfo) {
    let width = metadata.width as i32;
    let height = metadata.height as i32;
    let layer_width = layer.w as i32;
    let layer_height = layer.h as i32;
    let opacity = (layer.opacity * layer.fill_opacity).clamp(0.0, 1.0);

    if opacity <= f32::EPSILON || layer_width <= 0 || layer_height <= 0 {
        return;
    }

    for layer_y in 0..layer_height {
        let dest_y = layer.y + layer_y;
        if dest_y < 0 || dest_y >= height {
            continue;
        }

        for layer_x in 0..layer_width {
            let dest_x = layer.x + layer_x;
            if dest_x < 0 || dest_x >= width {
                continue;
            }

            let src_index = ((layer_y * layer_width + layer_x) * 4) as usize;
            if src_index + 3 >= layer.image_data_rgba.len() {
                continue;
            }

            let src = source_pixel(metadata, layer, src_index);
            if src[3] == 0 {
                continue;
            }

            let dest_index = ((dest_y * width + dest_x) * 4) as usize;
            let dst = [
                canvas[dest_index],
                canvas[dest_index + 1],
                canvas[dest_index + 2],
                canvas[dest_index + 3],
            ];
            let out = blend_pixel(dst, src, opacity);
            canvas[dest_index] = out[0];
            canvas[dest_index + 1] = out[1];
            canvas[dest_index + 2] = out[2];
            canvas[dest_index + 3] = out[3];
        }
    }
}

fn source_pixel(metadata: &PsdMetadata, layer: &LayerInfo, src_index: usize) -> [u8; 4] {
    match metadata.color_mode {
        COLOR_MODE_GRAYSCALE => {
            let gray = layer.image_data_rgba[src_index];
            [gray, gray, gray, layer.image_data_rgba[src_index + 3]]
        }
        COLOR_MODE_CMYK => cmyk_to_rgba(layer, src_index),
        COLOR_MODE_RGB => [
            layer.image_data_rgba[src_index],
            layer.image_data_rgba[src_index + 1],
            layer.image_data_rgba[src_index + 2],
            layer.image_data_rgba[src_index + 3],
        ],
        _ => [
            layer.image_data_rgba[src_index],
            layer.image_data_rgba[src_index + 1],
            layer.image_data_rgba[src_index + 2],
            layer.image_data_rgba[src_index + 3],
        ],
    }
}

fn cmyk_to_rgba(layer: &LayerInfo, src_index: usize) -> [u8; 4] {
    let c = layer.image_data_rgba[src_index] as u16;
    let m = layer.image_data_rgba[src_index + 1] as u16;
    let y = layer.image_data_rgba[src_index + 2] as u16;
    let alpha = layer.image_data_rgba[src_index + 3];
    let k_index = src_index / 4;
    let k = layer.image_data_k.get(k_index).copied().unwrap_or_default() as u16;

    let r = 255u16.saturating_sub((c + k).min(255)) as u8;
    let g = 255u16.saturating_sub((m + k).min(255)) as u8;
    let b = 255u16.saturating_sub((y + k).min(255)) as u8;

    [r, g, b, alpha]
}

fn float_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
