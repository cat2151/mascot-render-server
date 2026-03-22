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
