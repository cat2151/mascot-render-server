use std::path::Path;

use anyhow::{bail, Context, Result};
use mascot_render_core::{load_mascot_config, write_mascot_config, MascotConfig, MascotTarget};

pub(crate) fn persist_requested_character_change(
    config_path: &Path,
    config: &MascotConfig,
) -> Result<()> {
    write_mascot_config(config_path, &character_target(config))
}

pub(crate) fn verify_persisted_character_change(
    config_path: &Path,
    expected: &MascotConfig,
) -> Result<MascotTarget> {
    let expected_target = character_target(expected);
    let persisted_config = load_mascot_config(config_path)
        .with_context(|| format!("failed to reload {}", config_path.display()))?;
    let persisted = character_target(&persisted_config);
    if persisted != expected_target {
        bail!(
            "persisted mascot runtime state did not match the requested character source: requested_png={} persisted_png={} requested_zip={} persisted_zip={} requested_psd={} persisted_psd={} requested_display_diff={} persisted_display_diff={}",
            expected_target.png_path.display(),
            persisted.png_path.display(),
            expected_target.zip_path.display(),
            persisted.zip_path.display(),
            expected_target.psd_path_in_zip.display(),
            persisted.psd_path_in_zip.display(),
            optional_path_text(expected_target.display_diff_path.as_deref()),
            optional_path_text(persisted.display_diff_path.as_deref())
        );
    }
    Ok(persisted)
}

fn character_target(config: &MascotConfig) -> MascotTarget {
    MascotTarget {
        png_path: config.png_path.clone(),
        scale: config.scale,
        favorite_ensemble_scale: config.favorite_ensemble_scale,
        zip_path: config.zip_path.clone(),
        psd_path_in_zip: config.psd_path_in_zip.clone(),
        display_diff_path: config.display_diff_path.clone(),
    }
}

fn optional_path_text(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
pub(crate) use persist_requested_character_change as persist_requested_character_change_for_test;
#[cfg(test)]
pub(crate) use verify_persisted_character_change as verify_persisted_character_change_for_test;
