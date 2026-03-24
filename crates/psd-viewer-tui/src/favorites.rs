use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{local_data_root, ZipEntry};
use serde::{Deserialize, Serialize};

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "psd-viewer-tui.toml";
const FAVORITES_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FavoriteEntry {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) psd_file_name: String,
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
}

impl PartialEq for FavoriteEntry {
    fn eq(&self, other: &Self) -> bool {
        self.zip_path == other.zip_path && self.psd_path_in_zip == other.psd_path_in_zip
    }
}

impl Eq for FavoriteEntry {}

impl Hash for FavoriteEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zip_path.hash(state);
        self.psd_path_in_zip.hash(state);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct FavoritesFile {
    version: u32,
    favorites: Vec<FavoriteEntry>,
}

impl Default for FavoritesFile {
    fn default() -> Self {
        Self {
            version: FAVORITES_FILE_VERSION,
            favorites: Vec::new(),
        }
    }
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
        Ok(file) if file.version == FAVORITES_FILE_VERSION => {
            Ok(sanitize_favorites(file.favorites))
        }
        Ok(_) => Ok(Vec::new()),
        Err(_) => Ok(Vec::new()),
    }
}

pub(crate) fn save_favorites(path: &Path, favorites: &[FavoriteEntry]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let file = FavoritesFile {
        version: FAVORITES_FILE_VERSION,
        favorites: sanitize_favorites(favorites.to_vec()),
    };
    let toml = toml::to_string_pretty(&file).context("failed to serialize favorites")?;
    fs::write(path, toml).with_context(|| format!("failed to write favorites {}", path.display()))
}

#[cfg(test)]
pub(crate) fn favorite_selection(
    zip_entries: &[ZipEntry],
    favorite: &FavoriteEntry,
) -> Option<(usize, usize)> {
    favorite_selection_lookup(zip_entries)
        .get(&favorite.key())
        .copied()
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
    let mut seen = HashSet::new();
    let mut sanitized = Vec::new();
    for mut favorite in favorites {
        if favorite.psd_file_name.is_empty() {
            favorite.psd_file_name = favorite
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| favorite.psd_path_in_zip.display().to_string());
        }
        if seen.insert(favorite.key()) {
            sanitized.push(favorite);
        }
    }
    sanitized
}
