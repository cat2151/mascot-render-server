use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const RGBA_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaCacheImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub png_file_bytes: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RgbaCacheLoadReport {
    pub cache_hit: bool,
    pub status: String,
    pub meta_read_ms: u64,
    pub data_read_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RgbaCacheFile {
    version: u32,
    source: RgbaCacheSource,
    image_size: [u32; 2],
    rgba_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RgbaCacheSource {
    png_modified_unix_nanos: u64,
    png_file_bytes: u64,
}

pub(crate) fn write_default_rgba_cache_for_rgba(
    png_path: &Path,
    image_size: [u32; 2],
    rgba: &[u8],
) -> Result<()> {
    let expected_bytes = expected_rgba_bytes(image_size)?;
    if rgba.len() != expected_bytes {
        anyhow::bail!(
            "default rgba length does not match image size: path={} image_size={}x{} rgba_bytes={} expected_rgba_bytes={}",
            png_path.display(),
            image_size[0],
            image_size[1],
            rgba.len(),
            expected_bytes
        );
    }

    let meta_path = rgba_cache_meta_path(png_path);
    let data_path = rgba_cache_data_path(png_path);
    if let Some(parent) = meta_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&data_path, rgba)
        .with_context(|| format!("failed to write {}", data_path.display()))?;
    let meta = RgbaCacheFile {
        version: RGBA_CACHE_VERSION,
        source: rgba_cache_source(png_path)?,
        image_size,
        rgba_bytes: rgba.len(),
    };
    let json = serde_json::to_string_pretty(&meta).context("failed to serialize rgba cache")?;
    fs::write(&meta_path, json).with_context(|| format!("failed to write {}", meta_path.display()))
}

pub fn load_rgba_cache(png_path: &Path) -> Result<(Option<RgbaCacheImage>, RgbaCacheLoadReport)> {
    let meta_started_at = Instant::now();
    let Some(meta) = read_rgba_cache_meta(png_path)? else {
        return Ok((
            None,
            RgbaCacheLoadReport {
                status: "missing".to_string(),
                meta_read_ms: elapsed_ms_since(meta_started_at),
                ..RgbaCacheLoadReport::default()
            },
        ));
    };
    let meta_read_ms = elapsed_ms_since(meta_started_at);

    let source = match rgba_cache_source(png_path) {
        Ok(source) => source,
        Err(_) => return Ok((None, miss_report("source_unavailable", meta_read_ms))),
    };
    if !rgba_cache_meta_matches(&meta, &source) {
        return Ok((None, miss_report("stale", meta_read_ms)));
    }

    let data_path = rgba_cache_data_path(png_path);
    let data_started_at = Instant::now();
    let rgba = match fs::read(&data_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((None, miss_report("data_missing", meta_read_ms)));
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", data_path.display()));
        }
    };
    let data_read_ms = elapsed_ms_since(data_started_at);
    if rgba.len() != meta.rgba_bytes {
        return Ok((
            None,
            RgbaCacheLoadReport {
                status: "data_size_mismatch".to_string(),
                meta_read_ms,
                data_read_ms,
                ..RgbaCacheLoadReport::default()
            },
        ));
    }

    Ok((
        Some(RgbaCacheImage {
            width: meta.image_size[0],
            height: meta.image_size[1],
            rgba,
            png_file_bytes: usize::try_from(source.png_file_bytes).unwrap_or(usize::MAX),
        }),
        RgbaCacheLoadReport {
            cache_hit: true,
            status: "hit".to_string(),
            meta_read_ms,
            data_read_ms,
        },
    ))
}

pub fn rgba_cache_exists(png_path: &Path) -> bool {
    rgba_cache_is_complete(png_path).unwrap_or(false)
}

pub fn rgba_cache_meta_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("rgba.json")
}

pub fn rgba_cache_data_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("rgba")
}

fn miss_report(status: &str, meta_read_ms: u64) -> RgbaCacheLoadReport {
    RgbaCacheLoadReport {
        status: status.to_string(),
        meta_read_ms,
        ..RgbaCacheLoadReport::default()
    }
}

fn read_rgba_cache_meta(png_path: &Path) -> Result<Option<RgbaCacheFile>> {
    let meta_path = rgba_cache_meta_path(png_path);
    let meta_bytes = match fs::read(&meta_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", meta_path.display()));
        }
    };
    match serde_json::from_slice::<RgbaCacheFile>(&meta_bytes) {
        Ok(meta) => Ok(Some(meta)),
        Err(_) => Ok(None),
    }
}

fn rgba_cache_is_complete(png_path: &Path) -> Result<bool> {
    let Some(meta) = read_rgba_cache_meta(png_path)? else {
        return Ok(false);
    };
    let source = rgba_cache_source(png_path)?;
    if !rgba_cache_meta_matches(&meta, &source) {
        return Ok(false);
    }
    let data_path = rgba_cache_data_path(png_path);
    let data_metadata = match fs::metadata(&data_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to stat {}", data_path.display()));
        }
    };
    Ok(data_metadata.len() == meta.rgba_bytes as u64)
}

fn rgba_cache_meta_matches(meta: &RgbaCacheFile, source: &RgbaCacheSource) -> bool {
    meta.version == RGBA_CACHE_VERSION
        && meta.source == *source
        && expected_rgba_bytes(meta.image_size)
            .ok()
            .is_some_and(|bytes| bytes == meta.rgba_bytes)
}

fn rgba_cache_source(png_path: &Path) -> Result<RgbaCacheSource> {
    let metadata =
        fs::metadata(png_path).with_context(|| format!("failed to stat {}", png_path.display()))?;
    Ok(RgbaCacheSource {
        png_modified_unix_nanos: system_time_unix_nanos(metadata.modified().ok()),
        png_file_bytes: metadata.len(),
    })
}

fn system_time_unix_nanos(system_time: Option<SystemTime>) -> u64 {
    system_time
        .and_then(|system_time| system_time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
        .unwrap_or(0)
}

fn expected_rgba_bytes(image_size: [u32; 2]) -> Result<usize> {
    let pixels = u64::from(image_size[0])
        .checked_mul(u64::from(image_size[1]))
        .context("rgba image size overflows pixel count")?;
    let bytes = pixels
        .checked_mul(4)
        .context("rgba image size overflows byte count")?;
    usize::try_from(bytes).context("rgba image byte count does not fit usize")
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}
