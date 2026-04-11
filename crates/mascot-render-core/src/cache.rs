use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::archive::{collect_psd_files, collect_zip_files, extract_zip_to_dir};
use crate::cache_progress::{PsdLoadProgress, ZipLoadEvent, ZipLoadProgress};
use crate::model::{PsdEntry, ZipEntry};
use crate::psd::build_psd_entry;
use crate::workspace_paths::workspace_cache_root;

pub fn default_cache_root() -> PathBuf {
    workspace_cache_root()
}

const ZIP_META_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ZipSourceStamp {
    pub(crate) file_name: String,
    pub(crate) modified_unix_nanos: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZipMetaFile {
    version: u32,
    zip_path: PathBuf,
    zip_cache_key: String,
    source: ZipSourceStamp,
    psds: Vec<PsdEntry>,
    updated_at: u64,
}

pub(crate) fn load_zip_entries(
    zip_sources: &[PathBuf],
    cache_root: &Path,
) -> Result<Vec<ZipEntry>> {
    load_zip_entries_incremental(zip_sources, cache_root, |_| {})
}

pub(crate) fn load_zip_entries_incremental(
    zip_sources: &[PathBuf],
    cache_root: &Path,
    mut on_event: impl FnMut(ZipLoadEvent),
) -> Result<Vec<ZipEntry>> {
    fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create cache root {}", cache_root.display()))?;

    let zip_files = collect_zip_files(zip_sources)?;
    let mut zip_entries = Vec::with_capacity(zip_files.len());

    for zip_path in zip_files {
        zip_entries.push(load_zip_entry_incremental(
            &zip_path,
            cache_root,
            &mut on_event,
        )?);
    }

    on_event(ZipLoadEvent::Finished(zip_entries.clone()));
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
        if !snapshot_meta_is_usable(&cache_dir, &meta) {
            continue;
        }
        let psds = meta.psds;
        if psds.is_empty() {
            continue;
        }
        let zip_cache_key = meta.zip_cache_key;
        let zip_path = meta.zip_path;

        zip_entries.push(ZipEntry {
            zip_path: zip_path.clone(),
            zip_cache_key,
            cache_dir: cache_dir.clone(),
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
    load_zip_entry_incremental(zip_path, cache_root, |_| {})
}

fn load_zip_entry_incremental(
    zip_path: &Path,
    cache_root: &Path,
    mut on_event: impl FnMut(ZipLoadEvent),
) -> Result<ZipEntry> {
    let source = zip_source_stamp(zip_path)?;
    let zip_cache_key = zip_cache_key(&source);
    let cache_dir = cache_root.join(&zip_cache_key);
    let extracted_dir = cache_dir.join("extracted");
    let psd_meta_path = cache_dir.join("psd-meta.json");
    let progress = ZipLoadProgress {
        zip_path: zip_path.to_path_buf(),
        zip_cache_key: zip_cache_key.clone(),
        cache_dir: cache_dir.clone(),
        extracted_dir: extracted_dir.clone(),
        psd_meta_path: psd_meta_path.clone(),
    };

    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create cache dir {}", cache_dir.display()))?;
    on_event(ZipLoadEvent::ZipStarted(progress.clone()));

    let cached_meta = load_zip_meta_file(&psd_meta_path)?;
    let meta_is_reusable = cached_meta
        .as_ref()
        .is_some_and(|meta| zip_meta_is_reusable(meta, zip_path, &zip_cache_key, &source));

    if !meta_is_reusable {
        if extracted_dir.exists() {
            fs::remove_dir_all(&extracted_dir)
                .with_context(|| format!("failed to remove {}", extracted_dir.display()))?;
        }
        extract_zip_to_dir(zip_path, &extracted_dir)?;
        on_event(ZipLoadEvent::ZipExtracted(progress.clone()));
    }

    let (psds, updated_at) = match cached_meta {
        Some(meta) if meta_is_reusable => (meta.psds, meta.updated_at),
        _ => rebuild_zip_meta(
            ZipMetaBuildContext {
                zip_path,
                zip_cache_key: &zip_cache_key,
                source: &source,
                cache_dir: &cache_dir,
                extracted_dir: &extracted_dir,
                psd_meta_path: &psd_meta_path,
                progress: &progress,
            },
            &mut on_event,
        )?,
    };

    let entry = ZipEntry {
        zip_path: zip_path.to_path_buf(),
        zip_cache_key,
        cache_dir,
        extracted_dir,
        psd_meta_path,
        psds,
        updated_at,
    };
    on_event(ZipLoadEvent::ZipReady(entry.clone()));
    Ok(entry)
}

struct ZipMetaBuildContext<'a> {
    zip_path: &'a Path,
    zip_cache_key: &'a str,
    source: &'a ZipSourceStamp,
    cache_dir: &'a Path,
    extracted_dir: &'a Path,
    psd_meta_path: &'a Path,
    progress: &'a ZipLoadProgress,
}

fn rebuild_zip_meta(
    context: ZipMetaBuildContext<'_>,
    on_event: &mut impl FnMut(ZipLoadEvent),
) -> Result<(Vec<PsdEntry>, u64)> {
    let render_root = context.cache_dir.join("renders");
    fs::create_dir_all(&render_root)
        .with_context(|| format!("failed to create {}", render_root.display()))?;

    let mut psds = Vec::new();
    for path in collect_psd_files(context.extracted_dir)? {
        let progress = PsdLoadProgress {
            zip: context.progress.clone(),
            file_name: source_file_name(&path),
            psd_path: path.clone(),
        };
        on_event(ZipLoadEvent::PsdDiscovered(progress.clone()));
        let psd = build_psd_entry(&path, &render_root);
        on_event(ZipLoadEvent::PsdReady(progress, Box::new(psd.clone())));
        psds.push(psd);
    }
    psds.sort_by(|left, right| left.path.cmp(&right.path));

    let updated_at = unix_timestamp();
    let meta = ZipMetaFile {
        version: ZIP_META_VERSION,
        zip_path: context.zip_path.to_path_buf(),
        zip_cache_key: context.zip_cache_key.to_string(),
        source: context.source.clone(),
        psds: psds.clone(),
        updated_at,
    };
    write_zip_meta_file(context.psd_meta_path, &meta)?;

    Ok((psds, updated_at))
}

fn zip_meta_is_reusable(
    meta: &ZipMetaFile,
    zip_path: &Path,
    zip_cache_key: &str,
    source: &ZipSourceStamp,
) -> bool {
    meta.version == ZIP_META_VERSION
        && meta.zip_cache_key == zip_cache_key
        && meta.zip_path == zip_path
        && meta.source == *source
}

fn snapshot_meta_is_usable(cache_dir: &Path, meta: &ZipMetaFile) -> bool {
    meta.version == ZIP_META_VERSION
        && cache_dir
            .file_name()
            .is_some_and(|name| name == meta.zip_cache_key.as_str())
        && meta.zip_cache_key == zip_cache_key(&meta.source)
        && zip_source_stamp(&meta.zip_path)
            .ok()
            .is_some_and(|source| source == meta.source)
}

pub(crate) fn zip_source_stamp(path: &Path) -> Result<ZipSourceStamp> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to read metadata {}", path.display()))?;
    Ok(ZipSourceStamp {
        file_name: source_file_name(path),
        modified_unix_nanos: metadata.modified().ok().and_then(system_time_unix_nanos),
    })
}

fn zip_cache_key(source: &ZipSourceStamp) -> String {
    let name = sanitize_cache_component(&source.file_name);
    let timestamp = source
        .modified_unix_nanos
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!("{name}__mtime_{timestamp}")
}

fn source_file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn system_time_unix_nanos(time: SystemTime) -> Option<u64> {
    let nanos = time.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    u64::try_from(nanos).ok()
}

fn sanitize_cache_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "zip".to_string()
    } else {
        sanitized
    }
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

#[cfg(test)]
pub(crate) fn zip_cache_key_for_test(path: &Path) -> Result<String> {
    Ok(zip_cache_key(&zip_source_stamp(path)?))
}

#[cfg(test)]
pub(crate) fn zip_source_stamp_for_test(path: &Path) -> Result<ZipSourceStamp> {
    zip_source_stamp(path)
}
