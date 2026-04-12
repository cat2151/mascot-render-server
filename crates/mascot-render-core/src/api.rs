use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::LayerKind;

pub const VARIATION_SPEC_VERSION: u32 = 1;
pub const DISPLAY_DIFF_VERSION: u32 = VARIATION_SPEC_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdSummary {
    pub path_in_zip: PathBuf,
    pub file_name: String,
    pub metadata: String,
    pub error: Option<String>,
    pub default_rendered_png_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerDescriptor {
    pub layer_index: usize,
    pub name: String,
    pub kind: LayerKind,
    pub default_visible: bool,
    pub effective_visible: bool,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdDocument {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub file_name: String,
    pub metadata: String,
    pub layers: Vec<LayerDescriptor>,
    pub error: Option<String>,
    pub log_path: Option<PathBuf>,
    pub default_rendered_png_path: Option<PathBuf>,
    pub render_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VariationSpec {
    pub version: u32,
    pub visibility_overrides: Vec<LayerVisibilityOverride>,
}

impl VariationSpec {
    pub fn new() -> Self {
        Self {
            version: VARIATION_SPEC_VERSION,
            visibility_overrides: Vec::new(),
        }
    }

    pub fn is_default(&self) -> bool {
        self.visibility_overrides.is_empty()
    }
}

pub type DisplayDiff = VariationSpec;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerVisibilityOverride {
    pub layer_index: usize,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderRequest {
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub display_diff: DisplayDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderedPng {
    pub output_path: PathBuf,
    pub warnings: Vec<String>,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZipEntryLoadReport {
    pub elapsed_ms: u64,
    pub memory_cache_hit: bool,
    pub meta_cache_hit: bool,
    pub zip_extracted: bool,
    pub psd_meta_rebuilt: bool,
    pub psd_entries_built: usize,
    pub extract_ms: u64,
    pub psd_entry_build_ms: u64,
    pub rebuild_meta_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsdInspectReport {
    pub elapsed_ms: u64,
    pub zip_entry: ZipEntryLoadReport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPngReport {
    pub elapsed_ms: u64,
    pub zip_entry: ZipEntryLoadReport,
    pub default_render: bool,
    pub variation_cache_hit: bool,
    pub save_variation_spec_ms: u64,
    pub custom_psd_analyze_ms: u64,
    pub effective_visibility_ms: u64,
    pub compose_and_save_png_ms: u64,
    pub write_render_meta_ms: u64,
}
