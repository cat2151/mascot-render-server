use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::mascot::MascotImageData;

const SKIN_DETAILS_VERSION: u32 = 1;
const CONTENT_BOUNDS_ALPHA_THRESHOLD: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinContentBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl SkinContentBounds {
    pub fn full(image_size: [u32; 2]) -> Self {
        Self {
            min_x: 0,
            min_y: 0,
            max_x: image_size[0],
            max_y: image_size[1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkinDetails {
    pub alpha_mask: Arc<[u8]>,
    pub content_bounds: SkinContentBounds,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkinDetailsReport {
    pub cache_hit: bool,
    pub cache_read_ms: u64,
    pub alpha_mask_ms: u64,
    pub content_bounds_ms: u64,
    pub cache_write_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SkinDetailsCacheFile {
    version: u32,
    source: SkinDetailsSource,
    image_size: [u32; 2],
    alpha_mask_bytes: usize,
    content_bounds: SkinContentBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SkinDetailsSource {
    png_modified_unix_nanos: u64,
    png_file_bytes: u64,
}

pub fn load_or_build_skin_details(
    image: &MascotImageData,
) -> Result<(SkinDetails, SkinDetailsReport)> {
    let source = skin_details_source(&image.path)?;
    let image_size = [image.width, image.height];
    let expected_alpha_mask_bytes = expected_alpha_mask_bytes(image_size)?;

    let cache_read_started_at = Instant::now();
    if let Some(details) =
        load_skin_details_cache(&image.path, image_size, expected_alpha_mask_bytes, &source)?
    {
        return Ok((
            details,
            SkinDetailsReport {
                cache_hit: true,
                cache_read_ms: elapsed_ms_since(cache_read_started_at),
                ..SkinDetailsReport::default()
            },
        ));
    }
    let cache_read_ms = elapsed_ms_since(cache_read_started_at);

    let (details, mut report) = build_skin_details_from_rgba(&image.path, image_size, &image.rgba)?;
    report.cache_read_ms = cache_read_ms;

    let cache_write_started_at = Instant::now();
    write_skin_details_cache(&image.path, image_size, &source, &details)?;
    report.cache_write_ms = elapsed_ms_since(cache_write_started_at);

    Ok((details, report))
}

pub(crate) fn write_skin_details_cache_for_rgba(
    png_path: &Path,
    image_size: [u32; 2],
    rgba: &[u8],
) -> Result<SkinDetailsReport> {
    let (details, mut report) = build_skin_details_from_rgba(png_path, image_size, rgba)?;
    let source = skin_details_source(png_path)?;
    let cache_write_started_at = Instant::now();
    write_skin_details_cache(png_path, image_size, &source, &details)?;
    report.cache_write_ms = elapsed_ms_since(cache_write_started_at);
    Ok(report)
}

pub fn skin_details_cache_exists(png_path: &Path) -> bool {
    skin_details_cache_is_complete(png_path).unwrap_or(false)
}

pub fn skin_details_meta_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("skin.json")
}

pub fn skin_details_alpha_path(png_path: &Path) -> PathBuf {
    png_path.with_extension("alpha")
}

fn build_skin_details_from_rgba(
    png_path: &Path,
    image_size: [u32; 2],
    rgba: &[u8],
) -> Result<(SkinDetails, SkinDetailsReport)> {
    let alpha_mask_started_at = Instant::now();
    let expected_alpha_mask_bytes = expected_alpha_mask_bytes(image_size)?;
    let mut alpha_mask = Vec::with_capacity(expected_alpha_mask_bytes);
    let mut min_x = image_size[0];
    let mut min_y = image_size[1];
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    let threshold = CONTENT_BOUNDS_ALPHA_THRESHOLD.max(1);
    let width_usize = image_size[0] as usize;

    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        let alpha = pixel[3];
        alpha_mask.push(alpha);
        if alpha < threshold {
            continue;
        }

        let x = (index % width_usize) as u32;
        let y = (index / width_usize) as u32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + 1);
        max_y = max_y.max(y + 1);
        found = true;
    }
    let alpha_mask_ms = elapsed_ms_since(alpha_mask_started_at);

    if alpha_mask.len() != expected_alpha_mask_bytes {
        anyhow::bail!(
            "skin rgba length does not match image size: path={} image_size={}x{} rgba_bytes={} expected_alpha_mask_bytes={}",
            png_path.display(),
            image_size[0],
            image_size[1],
            rgba.len(),
            expected_alpha_mask_bytes
        );
    }

    let content_bounds_started_at = Instant::now();
    let content_bounds = if found {
        SkinContentBounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    } else {
        eprintln!(
            "mascot skin {:?} has no visible alpha region; keeping full image bounds",
            image_size
        );
        SkinContentBounds::full(image_size)
    };
    let content_bounds_ms = elapsed_ms_since(content_bounds_started_at);

    Ok((
        SkinDetails {
            alpha_mask: alpha_mask.into(),
            content_bounds,
        },
        SkinDetailsReport {
            alpha_mask_ms,
            content_bounds_ms,
            ..SkinDetailsReport::default()
        },
    ))
}

fn load_skin_details_cache(
    png_path: &Path,
    image_size: [u32; 2],
    expected_alpha_mask_bytes: usize,
    source: &SkinDetailsSource,
) -> Result<Option<SkinDetails>> {
    let meta = match read_skin_details_meta(png_path)? {
        Some(meta) => meta,
        None => return Ok(None),
    };
    if !skin_details_cache_matches(&meta, image_size, expected_alpha_mask_bytes, source) {
        return Ok(None);
    }

    let alpha_path = skin_details_alpha_path(png_path);
    let alpha_mask = match fs::read(&alpha_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", alpha_path.display()));
        }
    };
    if alpha_mask.len() != expected_alpha_mask_bytes {
        return Ok(None);
    }

    Ok(Some(SkinDetails {
        alpha_mask: alpha_mask.into(),
        content_bounds: meta.content_bounds,
    }))
}

fn write_skin_details_cache(
    png_path: &Path,
    image_size: [u32; 2],
    source: &SkinDetailsSource,
    details: &SkinDetails,
) -> Result<()> {
    let meta_path = skin_details_meta_path(png_path);
    let alpha_path = skin_details_alpha_path(png_path);
    if let Some(parent) = meta_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&alpha_path, details.alpha_mask.as_ref())
        .with_context(|| format!("failed to write {}", alpha_path.display()))?;
    let meta = SkinDetailsCacheFile {
        version: SKIN_DETAILS_VERSION,
        source: source.clone(),
        image_size,
        alpha_mask_bytes: details.alpha_mask.len(),
        content_bounds: details.content_bounds,
    };
    let json =
        serde_json::to_string_pretty(&meta).context("failed to serialize skin details cache")?;
    fs::write(&meta_path, json).with_context(|| format!("failed to write {}", meta_path.display()))
}

fn skin_details_cache_is_complete(png_path: &Path) -> Result<bool> {
    let Some(meta) = read_skin_details_meta(png_path)? else {
        return Ok(false);
    };
    if meta.version != SKIN_DETAILS_VERSION {
        return Ok(false);
    }
    let source = skin_details_source(png_path)?;
    if meta.source != source || !content_bounds_fit_image(meta.content_bounds, meta.image_size) {
        return Ok(false);
    }
    let alpha_path = skin_details_alpha_path(png_path);
    let alpha_metadata = match fs::metadata(&alpha_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to stat {}", alpha_path.display()));
        }
    };
    Ok(alpha_metadata.len() == meta.alpha_mask_bytes as u64)
}

fn read_skin_details_meta(png_path: &Path) -> Result<Option<SkinDetailsCacheFile>> {
    let meta_path = skin_details_meta_path(png_path);
    let meta_bytes = match fs::read(&meta_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", meta_path.display()));
        }
    };
    match serde_json::from_slice::<SkinDetailsCacheFile>(&meta_bytes) {
        Ok(meta) => Ok(Some(meta)),
        Err(_) => Ok(None),
    }
}

fn skin_details_cache_matches(
    meta: &SkinDetailsCacheFile,
    image_size: [u32; 2],
    expected_alpha_mask_bytes: usize,
    source: &SkinDetailsSource,
) -> bool {
    meta.version == SKIN_DETAILS_VERSION
        && meta.image_size == image_size
        && meta.alpha_mask_bytes == expected_alpha_mask_bytes
        && meta.source == *source
        && content_bounds_fit_image(meta.content_bounds, image_size)
}

fn content_bounds_fit_image(bounds: SkinContentBounds, image_size: [u32; 2]) -> bool {
    bounds.min_x <= bounds.max_x
        && bounds.min_y <= bounds.max_y
        && bounds.max_x <= image_size[0]
        && bounds.max_y <= image_size[1]
}

fn skin_details_source(png_path: &Path) -> Result<SkinDetailsSource> {
    let metadata =
        fs::metadata(png_path).with_context(|| format!("failed to stat {}", png_path.display()))?;
    Ok(SkinDetailsSource {
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

fn expected_alpha_mask_bytes(image_size: [u32; 2]) -> Result<usize> {
    let pixels = u64::from(image_size[0])
        .checked_mul(u64::from(image_size[1]))
        .context("skin image size overflows pixel count")?;
    usize::try_from(pixels).context("skin image pixel count does not fit usize")
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}
