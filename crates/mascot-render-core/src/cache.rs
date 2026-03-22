use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::archive::{collect_psd_files, collect_zip_files, extract_zip_to_dir};
use crate::model::{PsdEntry, ZipEntry};
use crate::psd::build_psd_entry;
use crate::workspace_paths::workspace_cache_root;

pub fn default_cache_root() -> PathBuf {
    workspace_cache_root()
}

const ZIP_META_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct ZipMetaFile {
    version: u32,
    zip_path: PathBuf,
    zip_hash: String,
    psds: Vec<PsdEntry>,
    updated_at: u64,
}

pub(crate) fn load_zip_entries(
    zip_sources: &[PathBuf],
    cache_root: &Path,
) -> Result<Vec<ZipEntry>> {
    fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create cache root {}", cache_root.display()))?;

    let zip_files = collect_zip_files(zip_sources)?;
    let mut zip_entries = Vec::with_capacity(zip_files.len());

    for zip_path in zip_files {
        zip_entries.push(load_zip_entry(&zip_path, cache_root)?);
    }

    Ok(zip_entries)
}

pub(crate) fn load_cached_zip_entries_snapshot(cache_root: &Path) -> Result<Vec<ZipEntry>> {
    if !cache_root.exists() {
        return Ok(Vec::new());
    }

    let mut zip_entries = Vec::new();
    for entry in fs::read_dir(cache_root)
        .with_context(|| format!("failed to read cache root {}", cache_root.display()))?
    {
        let entry = entry.with_context(|| format!("failed to iterate {}", cache_root.display()))?;
        let cache_dir = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let psd_meta_path = cache_dir.join("psd-meta.json");
        let Some(meta) = load_zip_meta_file(&psd_meta_path)? else {
            continue;
        };
        if !snapshot_meta_is_usable(&meta) {
            continue;
        }
        let psds = snapshot_psds(meta.psds);
        if psds.is_empty() {
            continue;
        }

        zip_entries.push(ZipEntry {
            zip_path: meta.zip_path,
            zip_hash: meta.zip_hash,
            cache_dir: cache_dir.clone(),
            source_zip_path: cache_dir.join("source.zip"),
            extracted_dir: cache_dir.join("extracted"),
            psd_meta_path,
            psds,
            updated_at: meta.updated_at,
        });
    }

    zip_entries.sort_by(|left, right| left.zip_path.cmp(&right.zip_path));
    Ok(zip_entries)
}

pub(crate) fn load_zip_entry(zip_path: &Path, cache_root: &Path) -> Result<ZipEntry> {
    let zip_hash = hash_file(zip_path)?;
    let cache_dir = cache_root.join(&zip_hash);
    let source_zip_path = cache_dir.join("source.zip");
    let extracted_dir = cache_dir.join("extracted");
    let psd_meta_path = cache_dir.join("psd-meta.json");

    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create cache dir {}", cache_dir.display()))?;
    ensure_source_zip(zip_path, &source_zip_path)?;

    let cached_meta = load_zip_meta_file(&psd_meta_path)?;
    let meta_is_reusable = cached_meta
        .as_ref()
        .is_some_and(|meta| zip_meta_is_reusable(meta, zip_path, &zip_hash));

    if !meta_is_reusable {
        if extracted_dir.exists() {
            fs::remove_dir_all(&extracted_dir)
                .with_context(|| format!("failed to remove {}", extracted_dir.display()))?;
        }
        extract_zip_to_dir(&source_zip_path, &extracted_dir)?;
    }

    let psds = match cached_meta {
        Some(meta) if meta_is_reusable => meta.psds,
        _ => rebuild_zip_meta(
            zip_path,
            &zip_hash,
            &cache_dir,
            &extracted_dir,
            &psd_meta_path,
        )?,
    };

    Ok(ZipEntry {
        zip_path: zip_path.to_path_buf(),
        zip_hash,
        cache_dir,
        source_zip_path,
        extracted_dir,
        psd_meta_path,
        psds,
        updated_at: unix_timestamp(),
    })
}

fn rebuild_zip_meta(
    zip_path: &Path,
    zip_hash: &str,
    cache_dir: &Path,
    extracted_dir: &Path,
    psd_meta_path: &Path,
) -> Result<Vec<PsdEntry>> {
    let render_root = cache_dir.join("renders");
    fs::create_dir_all(&render_root)
        .with_context(|| format!("failed to create {}", render_root.display()))?;

    let mut psds = collect_psd_files(extracted_dir)?
        .into_iter()
        .map(|path| build_psd_entry(&path, &render_root))
        .collect::<Vec<_>>();
    psds.sort_by(|left, right| left.path.cmp(&right.path));

    let meta = ZipMetaFile {
        version: ZIP_META_VERSION,
        zip_path: zip_path.to_path_buf(),
        zip_hash: zip_hash.to_string(),
        psds: psds.clone(),
        updated_at: unix_timestamp(),
    };
    write_zip_meta_file(psd_meta_path, &meta)?;

    Ok(psds)
}

fn zip_meta_is_reusable(meta: &ZipMetaFile, zip_path: &Path, zip_hash: &str) -> bool {
    meta.version == ZIP_META_VERSION
        && meta.zip_hash == zip_hash
        && meta.zip_path == zip_path
        && meta.psds.iter().all(psd_entry_is_reusable)
}

fn psd_entry_is_reusable(entry: &PsdEntry) -> bool {
    entry.path.exists()
        && match entry.rendered_png_path.as_ref() {
            Some(path) => path.exists(),
            None => true,
        }
}

fn snapshot_meta_is_usable(meta: &ZipMetaFile) -> bool {
    meta.version == ZIP_META_VERSION && meta.zip_path.exists()
}

fn snapshot_psds(psds: Vec<PsdEntry>) -> Vec<PsdEntry> {
    psds.into_iter()
        .filter_map(|mut psd| {
            if !psd.path.exists() {
                return None;
            }

            if psd
                .rendered_png_path
                .as_ref()
                .is_some_and(|path| !path.exists())
            {
                psd.rendered_png_path = None;
            }

            Some(psd)
        })
        .collect()
}

fn ensure_source_zip(zip_path: &Path, source_zip_path: &Path) -> Result<()> {
    if source_zip_path.exists() {
        return Ok(());
    }

    if let Some(parent) = source_zip_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::copy(zip_path, source_zip_path).with_context(|| {
        format!(
            "failed to copy {} to {}",
            zip_path.display(),
            source_zip_path.display()
        )
    })?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn load_zip_meta_file(path: &Path) -> Result<Option<ZipMetaFile>> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes =
        fs::read(path).with_context(|| format!("failed to read zip meta {}", path.display()))?;
    match serde_json::from_slice::<ZipMetaFile>(&bytes) {
        Ok(meta) => Ok(Some(meta)),
        Err(_) => Ok(None),
    }
}

fn write_zip_meta_file(path: &Path, meta: &ZipMetaFile) -> Result<()> {
    let json = serde_json::to_string_pretty(meta).context("failed to serialize zip meta")?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}
