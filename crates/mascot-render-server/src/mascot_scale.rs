use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use eframe::egui::Modifiers;
use mascot_render_core::{mascot_window_size, write_mascot_config, MascotConfig, MascotTarget};

pub(crate) const MASCOT_SCALE_STEP: f32 = 0.10;
pub(crate) const MIN_MASCOT_SCALE: f32 = 0.01;
pub(crate) const SCALE_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);

pub(crate) fn effective_scale(width: u32, height: u32, configured_scale: Option<f32>) -> f32 {
    let [scaled_width, scaled_height] = mascot_window_size(width, height, configured_scale);
    if width > 0 {
        return scaled_width / width as f32;
    }
    if height > 0 {
        return scaled_height / height as f32;
    }
    1.0
}

pub(crate) fn adjust_scale(current_scale: f32, steps: i32) -> Option<f32> {
    if steps == 0 || !current_scale.is_finite() || current_scale <= 0.0 {
        return None;
    }

    let next_scale = (current_scale + steps as f32 * MASCOT_SCALE_STEP).max(MIN_MASCOT_SCALE);
    ((next_scale - current_scale).abs() > f32::EPSILON).then_some(next_scale)
}

pub(crate) fn keyboard_scale_steps(
    modifiers: Modifiers,
    increase_pressed: bool,
    decrease_pressed: bool,
) -> i32 {
    if modifiers.alt || modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
        return 0;
    }

    increase_pressed as i32 - decrease_pressed as i32
}

pub(crate) fn scroll_scale_steps(scroll_delta_y: f32) -> i32 {
    if !scroll_delta_y.is_finite() || scroll_delta_y.abs() <= f32::EPSILON {
        return 0;
    }

    scroll_delta_y.signum() as i32
}

pub(crate) fn persist_scale(config_path: &Path, config: &MascotConfig, scale: f32) -> Result<()> {
    write_mascot_config(
        config_path,
        &MascotTarget {
            png_path: config.png_path.clone(),
            scale: Some(scale),
            favorite_ensemble_scale: config.favorite_ensemble_scale,
            zip_path: config.zip_path.clone(),
            psd_path_in_zip: config.psd_path_in_zip.clone(),
            display_diff_path: config.display_diff_path.clone(),
        },
    )
}

pub(crate) fn persist_favorite_ensemble_scale(
    config_path: &Path,
    config: &MascotConfig,
    scale: f32,
) -> Result<()> {
    write_mascot_config(
        config_path,
        &MascotTarget {
            png_path: config.png_path.clone(),
            scale: config.scale,
            favorite_ensemble_scale: Some(scale),
            zip_path: config.zip_path.clone(),
            psd_path_in_zip: config.psd_path_in_zip.clone(),
            display_diff_path: config.display_diff_path.clone(),
        },
    )
}
