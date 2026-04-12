use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::api::{
    PsdDocument, PsdInspectReport, PsdSummary, RenderPngReport, RenderRequest, RenderedPng,
    ZipEntryLoadReport,
};
use crate::cache::{
    default_cache_root, load_cached_zip_entries_snapshot, load_zip_entries,
    load_zip_entries_incremental, load_zip_entry,
    load_zip_entry_with_report as load_zip_entry_with_report_from_cache, zip_source_stamp,
    ZipSourceStamp,
};
use crate::cache_progress::ZipLoadEvent;
use crate::model::{PsdEntry, ZipEntry};
use crate::psd::{analyze_psd, effective_visibility_with_overrides};
use crate::render::{render_png as render_png_with_visibility, RenderSidecars};
use crate::variation::{
    save_variation_spec, variation_png_path, variation_render_meta_path, variation_spec_path,
};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

const CUSTOM_RENDER_META_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub cache_dir: PathBuf,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_root(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Core {
    cache_dir: PathBuf,
    zip_entry_cache: Arc<Mutex<HashMap<PathBuf, CachedZipEntry>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CustomRenderMeta {
    version: u32,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct CachedZipEntry {
    source: ZipSourceStamp,
    entry: ZipEntry,
}

impl Core {
    pub fn new(config: CoreConfig) -> Self {
        Self {
            cache_dir: config.cache_dir,
            zip_entry_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn load_zip_entries(&self, zip_sources: &[PathBuf]) -> Result<Vec<ZipEntry>> {
        let entries = load_zip_entries(zip_sources, &self.cache_dir)?;
        self.cache_zip_entries(&entries)?;
        Ok(entries)
    }

    pub fn load_zip_entries_incremental(
        &self,
        zip_sources: &[PathBuf],
        on_event: impl FnMut(ZipLoadEvent),
    ) -> Result<Vec<ZipEntry>> {
        let entries = load_zip_entries_incremental(zip_sources, &self.cache_dir, on_event)?;
        self.cache_zip_entries(&entries)?;
        Ok(entries)
    }

    pub fn load_cached_zip_entries_snapshot(&self) -> Result<Vec<ZipEntry>> {
        let entries = load_cached_zip_entries_snapshot(&self.cache_dir)?;
        self.cache_zip_entries(&entries)?;
        Ok(entries)
    }

    pub fn load_zip_entry(&self, zip_path: &Path) -> Result<ZipEntry> {
        let source = zip_source_stamp(zip_path)?;
        if let Some(entry) = self.cached_zip_entry(zip_path, &source)? {
            return Ok(entry);
        }

        let entry = load_zip_entry(zip_path, &self.cache_dir)?;
        self.cache_zip_entry(entry.clone(), source)?;
        Ok(entry)
    }

    pub fn list_psds(&self, zip_path: &Path) -> Result<Vec<PsdSummary>> {
        let zip_entry = self.load_zip_entry(zip_path)?;
        zip_entry
            .psds
            .iter()
            .map(|psd| psd_summary(&zip_entry, psd))
            .collect()
    }

    pub fn inspect_psd(&self, zip_path: &Path, psd_path_in_zip: &Path) -> Result<PsdDocument> {
        self.inspect_psd_with_report(zip_path, psd_path_in_zip)
            .map(|(document, _)| document)
    }

    pub fn inspect_psd_with_report(
        &self,
        zip_path: &Path,
        psd_path_in_zip: &Path,
    ) -> Result<(PsdDocument, PsdInspectReport)> {
        let started_at = Instant::now();
        let (zip_entry, zip_report) = self.load_zip_entry_with_report(zip_path)?;
        let normalized_path = normalize_relative_path(psd_path_in_zip)?;
        let psd_entry = find_psd_entry(&zip_entry, &normalized_path)?;

        Ok((
            psd_entry.to_document(&zip_entry.zip_path, &normalized_path),
            PsdInspectReport {
                elapsed_ms: elapsed_ms_since(started_at),
                zip_entry: zip_report,
            },
        ))
    }

    pub fn render_png(&self, request: RenderRequest) -> Result<RenderedPng> {
        self.render_png_with_report(request)
            .map(|(rendered, _)| rendered)
    }

    pub fn render_png_with_report(
        &self,
        request: RenderRequest,
    ) -> Result<(RenderedPng, RenderPngReport)> {
        let started_at = Instant::now();
        let (zip_entry, zip_report) = self.load_zip_entry_with_report(&request.zip_path)?;
        let mut report = RenderPngReport {
            zip_entry: zip_report,
            ..RenderPngReport::default()
        };
        let normalized_path = normalize_relative_path(&request.psd_path_in_zip)?;
        let psd_entry = find_psd_entry(&zip_entry, &normalized_path)?;
        if request.display_diff.is_default() {
            let output_path = psd_entry.rendered_png_path.clone().ok_or_else(|| {
                anyhow!(
                    "default render is unavailable for '{}'",
                    normalized_path.display()
                )
            })?;
            report.default_render = true;
            report.variation_cache_hit = true;
            report.elapsed_ms = elapsed_ms_since(started_at);
            return Ok((
                RenderedPng {
                    output_path,
                    warnings: psd_entry.render_warnings.clone(),
                    cache_hit: true,
                },
                report,
            ));
        }

        let output_path = variation_png_path(
            &zip_entry.cache_dir,
            &normalized_path,
            &psd_entry.file_name,
            &request.display_diff,
        );
        let spec_path = variation_spec_path(&output_path);
        let meta_path = variation_render_meta_path(&output_path);

        let save_spec_started_at = Instant::now();
        save_variation_spec(
            &spec_path,
            &zip_entry.zip_path,
            &normalized_path,
            &request.display_diff,
        )?;
        report.save_variation_spec_ms = elapsed_ms_since(save_spec_started_at);

        if output_path.exists() {
            report.variation_cache_hit = true;
            report.elapsed_ms = elapsed_ms_since(started_at);
            return Ok((
                RenderedPng {
                    output_path,
                    warnings: load_custom_render_warnings(&meta_path)?,
                    cache_hit: true,
                },
                report,
            ));
        }

        let analyze_started_at = Instant::now();
        let analysis = analyze_psd(&psd_entry.path);
        report.custom_psd_analyze_ms = elapsed_ms_since(analyze_started_at);
        if let Some(error) = analysis.visible_error() {
            let log_hint = psd_entry
                .log_path
                .as_ref()
                .map(|path| format!(" See {}.", path.display()))
                .unwrap_or_default();
            bail!(
                "failed to inspect PSD '{}': {}{}",
                normalized_path.display(),
                error,
                log_hint
            );
        }

        let metadata = analysis.metadata.as_ref().ok_or_else(|| {
            anyhow!(
                "PSD metadata is unavailable for '{}'",
                normalized_path.display()
            )
        })?;
        let visibility_started_at = Instant::now();
        let effective_visibility = effective_visibility_with_overrides(
            &analysis.layers,
            &request.display_diff.visibility_overrides,
        )?;
        report.effective_visibility_ms = elapsed_ms_since(visibility_started_at);
        let render_started_at = Instant::now();
        let render_result = render_png_with_visibility(
            metadata,
            &analysis.layers,
            &effective_visibility,
            &output_path,
            RenderSidecars::default(),
        )
        .map_err(|error| anyhow!("failed to render '{}': {error}", normalized_path.display()))?;
        report.compose_and_save_png_ms = elapsed_ms_since(render_started_at);

        let write_meta_started_at = Instant::now();
        write_custom_render_meta(&meta_path, &render_result.warnings)?;
        report.write_render_meta_ms = elapsed_ms_since(write_meta_started_at);
        report.elapsed_ms = elapsed_ms_since(started_at);

        Ok((
            RenderedPng {
                output_path: render_result.output_path,
                warnings: render_result.warnings,
                cache_hit: false,
            },
            report,
        ))
    }

    fn load_zip_entry_with_report(
        &self,
        zip_path: &Path,
    ) -> Result<(ZipEntry, ZipEntryLoadReport)> {
        let started_at = Instant::now();
        let source = zip_source_stamp(zip_path)?;
        if let Some(entry) = self.cached_zip_entry(zip_path, &source)? {
            return Ok((
                entry,
                ZipEntryLoadReport {
                    elapsed_ms: elapsed_ms_since(started_at),
                    memory_cache_hit: true,
                    ..ZipEntryLoadReport::default()
                },
            ));
        }

        let (entry, report) = load_zip_entry_with_report_from_cache(zip_path, &self.cache_dir)?;
        self.cache_zip_entry(entry.clone(), source)?;
        Ok((entry, report))
    }

    fn cached_zip_entry(
        &self,
        zip_path: &Path,
        source: &ZipSourceStamp,
    ) -> Result<Option<ZipEntry>> {
        let cache = self
            .zip_entry_cache
            .lock()
            .map_err(|_| anyhow!("zip entry cache lock was poisoned"))?;
        Ok(cache
            .get(zip_path)
            .filter(|cached| cached.source == *source)
            .map(|cached| cached.entry.clone()))
    }

    fn cache_zip_entry(&self, entry: ZipEntry, source: ZipSourceStamp) -> Result<()> {
        let mut cache = self
            .zip_entry_cache
            .lock()
            .map_err(|_| anyhow!("zip entry cache lock was poisoned"))?;
        cache.insert(entry.zip_path.clone(), CachedZipEntry { source, entry });
        Ok(())
    }

    fn cache_zip_entries(&self, entries: &[ZipEntry]) -> Result<()> {
        let mut cache = self
            .zip_entry_cache
            .lock()
            .map_err(|_| anyhow!("zip entry cache lock was poisoned"))?;
        for entry in entries {
            let source = zip_source_stamp(&entry.zip_path)?;
            cache.insert(
                entry.zip_path.clone(),
                CachedZipEntry {
                    source,
                    entry: entry.clone(),
                },
            );
        }
        Ok(())
    }
}

fn psd_summary(zip_entry: &ZipEntry, psd: &PsdEntry) -> Result<PsdSummary> {
    Ok(PsdSummary {
        path_in_zip: path_in_zip(&zip_entry.extracted_dir, &psd.path)?,
        file_name: psd.file_name.clone(),
        metadata: psd.metadata.clone(),
        error: psd.error.clone(),
        default_rendered_png_path: psd.rendered_png_path.clone(),
    })
}

fn find_psd_entry<'a>(zip_entry: &'a ZipEntry, psd_path_in_zip: &Path) -> Result<&'a PsdEntry> {
    zip_entry
        .psds
        .iter()
        .find(|psd| {
            path_in_zip(&zip_entry.extracted_dir, &psd.path)
                .map(|path| path == psd_path_in_zip)
                .unwrap_or(false)
        })
        .with_context(|| {
            format!(
                "PSD '{}' was not found in '{}'",
                psd_path_in_zip.display(),
                zip_entry.zip_path.display()
            )
        })
}

fn path_in_zip(extracted_dir: &Path, psd_path: &Path) -> Result<PathBuf> {
    psd_path
        .strip_prefix(extracted_dir)
        .map(Path::to_path_buf)
        .with_context(|| {
            format!(
                "failed to resolve '{}' relative to '{}'",
                psd_path.display(),
                extracted_dir.display()
            )
        })
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("invalid PSD path '{}'", path.display());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        bail!("PSD path must not be empty");
    }

    Ok(normalized)
}

fn load_custom_render_warnings(meta_path: &Path) -> Result<Vec<String>> {
    if !meta_path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read(meta_path)
        .with_context(|| format!("failed to read render metadata {}", meta_path.display()))?;
    match serde_json::from_slice::<CustomRenderMeta>(&bytes) {
        Ok(meta) if meta.version == CUSTOM_RENDER_META_VERSION => Ok(meta.warnings),
        Ok(_) => Ok(Vec::new()),
        Err(_) => Ok(Vec::new()),
    }
}

fn write_custom_render_meta(meta_path: &Path, warnings: &[String]) -> Result<()> {
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let meta = CustomRenderMeta {
        version: CUSTOM_RENDER_META_VERSION,
        warnings: warnings.to_vec(),
    };
    let json =
        serde_json::to_string_pretty(&meta).context("failed to serialize render metadata")?;
    fs::write(meta_path, json)
        .with_context(|| format!("failed to write render metadata {}", meta_path.display()))?;
    Ok(())
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}
