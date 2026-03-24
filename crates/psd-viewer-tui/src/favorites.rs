use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mascot_render_core::{local_data_root, ZipEntry};
use serde::{Deserialize, Serialize};

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "psd-viewer-tui.toml";
const FAVORITES_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct FavoriteEntry {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) psd_file_name: String,
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

pub(crate) fn favorite_selection(
    zip_entries: &[ZipEntry],
    favorite: &FavoriteEntry,
) -> Option<(usize, usize)> {
    zip_entries
        .iter()
        .enumerate()
        .find_map(|(zip_index, zip_entry)| {
            (zip_entry.zip_path == favorite.zip_path).then(|| {
                zip_entry
                    .psds
                    .iter()
                    .enumerate()
                    .find_map(|(psd_index, psd_entry)| {
                        let psd_path_in_zip = psd_entry
                            .path
                            .strip_prefix(&zip_entry.extracted_dir)
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|_| psd_entry.path.clone());
                        (psd_path_in_zip == favorite.psd_path_in_zip)
                            .then_some((zip_index, psd_index))
                    })
            })?
        })
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
        if !sanitized.contains(&favorite) {
            sanitized.push(favorite);
        }
    }
    sanitized
}
