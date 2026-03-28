use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::LayerVisibilityOverride;
use serde::{Deserialize, Serialize};

use super::{sanitize_mascot_scale, FavoriteEntry, FavoriteKey};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FavoriteEntryFile {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    #[serde(default)]
    psd_file_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    visibility_overrides: Vec<LayerVisibilityOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mascot_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_position: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    favorite_ensemble_position: Option<[f32; 2]>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteEntryFile>,
}

impl From<FavoriteEntryFile> for FavoriteEntry {
    fn from(value: FavoriteEntryFile) -> Self {
        let psd_file_name = if value.psd_file_name.is_empty() {
            value
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| value.psd_path_in_zip.display().to_string())
        } else {
            value.psd_file_name
        };
        Self {
            zip_path: value.zip_path,
            psd_path_in_zip: value.psd_path_in_zip,
            psd_file_name,
            mascot_scale: sanitize_mascot_scale(value.mascot_scale),
        }
    }
}

pub(crate) fn load_favorites(path: &Path) -> Result<Vec<FavoriteEntry>> {
    Ok(load_favorites_file(path)?
        .map(|file| sanitize_favorites(file.favorites))
        .unwrap_or_default())
}

pub(super) fn persist_scale_for_key(
    path: &Path,
    current_key: &FavoriteKey,
    sanitized_scale: Option<f32>,
) -> Result<bool> {
    let Some(mut favorites_file) = load_favorites_file(path)? else {
        return Ok(false);
    };

    let mut matched = false;
    for favorite in &mut favorites_file.favorites {
        if favorite.zip_path == current_key.zip_path
            && favorite.psd_path_in_zip == current_key.psd_path_in_zip
        {
            favorite.mascot_scale = sanitized_scale;
            matched = true;
        }
    }

    if !matched {
        return Ok(false);
    }

    save_favorites_file(path, &favorites_file)?;
    Ok(true)
}

fn sanitize_favorites(favorites: Vec<FavoriteEntryFile>) -> Vec<FavoriteEntry> {
    let mut seen = HashSet::new();
    let mut sanitized = Vec::new();
    for (index, favorite) in favorites.into_iter().map(FavoriteEntry::from).enumerate() {
        if favorite.zip_path.as_os_str().is_empty()
            || favorite.psd_path_in_zip.as_os_str().is_empty()
        {
            eprintln!(
                "favorite shuffle dropped empty-path favorite entry at index {}",
                index
            );
            continue;
        }
        if seen.insert(favorite.key()) {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn load_favorites_file(path: &Path) -> Result<Option<FavoritesFile>> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorites {}", path.display()))?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(Some(file)),
        Err(error) => {
            eprintln!(
                "favorite shuffle ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(None)
        }
    }
}

fn save_favorites_file(path: &Path, favorites_file: &FavoritesFile) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let toml = toml::to_string_pretty(favorites_file).context("failed to serialize favorites")?;
    fs::write(path, toml).with_context(|| format!("failed to write favorites {}", path.display()))
}
