use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use mascot_render_core::{load_mascot_config, write_mascot_config, MascotConfig, MascotTarget};

pub(crate) fn persist_requested_skin_change(
    config_path: &Path,
    config: &MascotConfig,
    png_path: &Path,
) -> Result<()> {
    write_mascot_config(config_path, &requested_skin_target(config, png_path))
}

/// Reloads the persisted mascot runtime state and returns the verified
/// `png_path` when it matches the requested skin change.
pub(crate) fn verify_persisted_skin_change(config_path: &Path, png_path: &Path) -> Result<PathBuf> {
    let persisted_png_path = load_mascot_config(config_path)
        .with_context(|| format!("failed to reload {}", config_path.display()))?
        .png_path;
    if persisted_png_path != png_path {
        bail!(
            "persisted mascot runtime state did not match the requested png_path: requested={} persisted={}",
            png_path.display(),
            persisted_png_path.display()
        );
    }
    Ok(persisted_png_path)
}

fn requested_skin_target(config: &MascotConfig, png_path: &Path) -> MascotTarget {
    MascotTarget {
        png_path: png_path.to_path_buf(),
        scale: config.scale,
        favorite_ensemble_scale: config.favorite_ensemble_scale,
        zip_path: config.zip_path.clone(),
        psd_path_in_zip: config.psd_path_in_zip.clone(),
        display_diff_path: config.display_diff_path.clone(),
    }
}

#[cfg(test)]
pub(crate) use persist_requested_skin_change as persist_requested_skin_change_for_test;
#[cfg(test)]
pub(crate) use verify_persisted_skin_change as verify_persisted_skin_change_for_test;
