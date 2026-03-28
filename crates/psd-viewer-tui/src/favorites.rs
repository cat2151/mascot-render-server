use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{local_data_root, LayerVisibilityOverride, ZipEntry};
use serde::{Deserialize, Serialize};

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "favorites.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct FavoriteEntry {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) psd_file_name: String,
    #[serde(default)]
    pub(crate) visibility_overrides: Vec<LayerVisibilityOverride>,
    #[serde(default)]
    pub(crate) mascot_scale: Option<f32>,
    #[serde(default)]
    pub(crate) window_position: Option<[f32; 2]>,
    #[serde(default)]
    pub(crate) favorite_ensemble_position: Option<[f32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FavoriteKey {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
}

impl FavoriteEntry {
    pub(crate) fn key(&self) -> FavoriteKey {
        FavoriteKey {
            zip_path: self.zip_path.clone(),
            psd_path_in_zip: self.psd_path_in_zip.clone(),
        }
    }

    pub(crate) fn same_favorite_identity_as(&self, other: &Self) -> bool {
        self.favorite_identity_key() == other.favorite_identity_key()
    }

    fn favorite_identity_key(&self) -> FavoriteIdentityKey {
        FavoriteIdentityKey {
            zip_path: self.zip_path.clone(),
            psd_path_in_zip: self.psd_path_in_zip.clone(),
            visibility_overrides: self
                .visibility_overrides
                .iter()
                .map(|layer| (layer.layer_index, layer.visible))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FavoriteIdentityKey {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    visibility_overrides: Vec<(usize, bool)>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteEntry>,
}

pub(crate) fn favorites_path() -> PathBuf {
    local_data_root()
        .join(FAVORITES_DIR)
        .join(FAVORITES_FILE_NAME)
}

pub(crate) fn load_favorites(path: &Path) -> Result<Vec<FavoriteEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorites {}", path.display()))?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(sanitize_favorites(file.favorites)),
        Err(_) => Ok(Vec::new()),
    }
}

pub(crate) fn save_favorites(path: &Path, favorites: &[FavoriteEntry]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let file = FavoritesFile {
        favorites: sanitize_favorites(favorites.to_vec()),
    };
    let toml = toml::to_string_pretty(&file).context("failed to serialize favorites")?;
    fs::write(path, toml).with_context(|| format!("failed to write favorites {}", path.display()))
}

pub(crate) fn favorite_selection_lookup(
    zip_entries: &[ZipEntry],
) -> HashMap<FavoriteKey, (usize, usize)> {
    let mut lookup = HashMap::new();
    for (zip_index, zip_entry) in zip_entries.iter().enumerate() {
        for (psd_index, psd_entry) in zip_entry.psds.iter().enumerate() {
            let psd_path_in_zip = if psd_entry.path.starts_with(&zip_entry.extracted_dir) {
                psd_entry
                    .path
                    .strip_prefix(&zip_entry.extracted_dir)
                    .map(Path::to_path_buf)
                    .expect("path must be relative to extracted_dir when starts_with succeeds")
            } else {
                psd_entry.path.clone()
            };
            lookup.insert(
                FavoriteKey {
                    zip_path: zip_entry.zip_path.clone(),
                    psd_path_in_zip,
                },
                (zip_index, psd_index),
            );
        }
    }
    lookup
}

fn sanitize_favorites(favorites: Vec<FavoriteEntry>) -> Vec<FavoriteEntry> {
    let mut sanitized = Vec::new();
    for mut favorite in favorites {
        if favorite.psd_file_name.is_empty() {
            favorite.psd_file_name = favorite
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| favorite.psd_path_in_zip.display().to_string());
        }
        favorite.mascot_scale = sanitize_mascot_scale(favorite.mascot_scale);
        favorite.window_position = sanitize_window_position(favorite.window_position);
        favorite.favorite_ensemble_position =
            sanitize_window_position(favorite.favorite_ensemble_position);
        let identity = favorite.favorite_identity_key();
        if let Some(index) = sanitized
            .iter()
            .position(|saved: &FavoriteEntry| saved.favorite_identity_key() == identity)
        {
            sanitized[index] = favorite;
        } else {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn sanitize_mascot_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn sanitize_window_position(position: Option<[f32; 2]>) -> Option<[f32; 2]> {
    position.filter(|[x, y]| x.is_finite() && y.is_finite())
}
