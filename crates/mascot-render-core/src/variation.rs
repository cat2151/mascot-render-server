use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::VariationSpec;

const SAVED_VARIATION_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedVariationSpec {
    version: u32,
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    variation: VariationSpec,
}

pub fn variation_hash(variation: &VariationSpec) -> String {
    let mut hasher = Sha256::new();
    let json = serde_json::to_vec(variation).unwrap_or_default();
    hasher.update(json);
    let digest = hasher.finalize();
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn variation_png_path(
    zip_cache_dir: &Path,
    psd_path_in_zip: &Path,
    file_name: &str,
    variation: &VariationSpec,
) -> PathBuf {
    let hash = variation_hash(variation);
    variation_dir(zip_cache_dir, psd_path_in_zip, file_name).join(format!("{hash}.png"))
}

pub fn variation_spec_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("json")
}

pub fn variation_render_meta_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("render.json")
}

pub fn save_variation_spec(
    path: &Path,
    zip_path: &Path,
    psd_path_in_zip: &Path,
    variation: &VariationSpec,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let saved = SavedVariationSpec {
        version: SAVED_VARIATION_VERSION,
        zip_path: zip_path.to_path_buf(),
        psd_path_in_zip: psd_path_in_zip.to_path_buf(),
        variation: variation.clone(),
    };
    let json = serde_json::to_string_pretty(&saved).context("failed to serialize variation")?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn load_variation_spec(
    path: &Path,
    zip_path: &Path,
    psd_path_in_zip: &Path,
) -> Option<VariationSpec> {
    let bytes = fs::read(path).ok()?;
    let saved = serde_json::from_slice::<SavedVariationSpec>(&bytes).ok()?;
    if saved.version != SAVED_VARIATION_VERSION {
        return None;
    }
    if saved.zip_path != zip_path || saved.psd_path_in_zip != psd_path_in_zip {
        return None;
    }
    Some(saved.variation)
}

fn variation_dir(zip_cache_dir: &Path, psd_path_in_zip: &Path, file_name: &str) -> PathBuf {
    let psd_hash = hash_psd_path(psd_path_in_zip);
    let base_name = sanitize_file_name(file_name.trim_end_matches(".psd"));
    zip_cache_dir
        .join("variations")
        .join(format!("{base_name}__{psd_hash}"))
}

fn hash_psd_path(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized = file_name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "variation".to_string()
    } else {
        sanitized
    }
}
