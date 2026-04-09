use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use mascot_render_core::{mascot_window_size, MascotConfig, MascotImageData};
use mascot_render_server::{alpha_bounds_from_mask, AlphaBounds};

const CONTENT_BOUNDS_ALPHA_THRESHOLD: u8 = 1;

#[derive(Clone)]
pub(crate) struct CachedSkin {
    pub(crate) path: PathBuf,
    pub(crate) texture: TextureHandle,
    pub(crate) image_size: [u32; 2],
    pub(crate) alpha_mask: Arc<[u8]>,
    pub(crate) content_bounds: AlphaBounds,
}

pub(crate) fn window_title(config: &MascotConfig, config_path: &Path) -> String {
    if config.favorite_ensemble_enabled {
        return format!(
            "Mascot Render Server: favorite ensemble ({})",
            config_path
                .file_name()
                .unwrap_or(config_path.as_os_str())
                .to_string_lossy()
        );
    }
    format!(
        "Mascot Render Server: {} ({})",
        config.psd_path_in_zip.display(),
        config_path
            .file_name()
            .unwrap_or(config_path.as_os_str())
            .to_string_lossy()
    )
}

pub(crate) fn cached_skin_from_image(ctx: &egui::Context, image: &MascotImageData) -> CachedSkin {
    let alpha_mask = alpha_mask(&image.rgba);
    let content_bounds = content_bounds([image.width, image.height], alpha_mask.as_ref());
    CachedSkin {
        path: image.path.clone(),
        texture: load_texture(ctx, image),
        image_size: [image.width, image.height],
        alpha_mask,
        content_bounds,
    }
}

pub(crate) fn size_vec(width: u32, height: u32, scale: Option<f32>) -> Vec2 {
    let [width, height] = mascot_window_size(width, height, scale);
    Vec2::new(width, height)
}

pub(crate) fn path_modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn load_texture(ctx: &egui::Context, image: &MascotImageData) -> TextureHandle {
    let color_image = ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    let texture_name = format!("mascot-png:{}", image.path.display());
    ctx.load_texture(texture_name, color_image, TextureOptions::LINEAR)
}

pub(crate) fn alpha_mask(rgba: &[u8]) -> Arc<[u8]> {
    rgba.chunks_exact(4)
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>()
        .into()
}

pub(crate) fn content_bounds(image_size: [u32; 2], alpha_mask: &[u8]) -> AlphaBounds {
    alpha_bounds_from_mask(image_size, alpha_mask, CONTENT_BOUNDS_ALPHA_THRESHOLD).unwrap_or_else(
        || {
            eprintln!(
                "mascot skin {:?} has no visible alpha region or an invalid alpha mask; keeping full image bounds",
                image_size
            );
            AlphaBounds::full(image_size)
        },
    )
}
