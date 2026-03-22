use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::{LayerDescriptor, PsdDocument};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayerKind {
    GroupOpen,
    GroupClose,
    #[default]
    Layer,
}

impl LayerKind {
    pub fn tag(self) -> &'static str {
        match self {
            Self::GroupOpen => "[Group+]",
            Self::GroupClose => "[Group-]",
            Self::Layer => "[Layer]",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerNode {
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    pub depth: usize,
}

impl LayerNode {
    pub fn display_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let visibility = if self.visible { "visible" } else { "hidden" };
        format!(
            "{}{} {} ({})",
            indent,
            self.kind.tag(),
            self.name,
            visibility
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PsdEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub metadata: String,
    pub layer_nodes: Vec<LayerNode>,
    #[serde(default)]
    pub layer_descriptors: Vec<LayerDescriptor>,
    pub error: Option<String>,
    pub log_path: Option<PathBuf>,
    pub rendered_png_path: Option<PathBuf>,
    pub render_warnings: Vec<String>,
    pub updated_at: u64,
}

impl PsdEntry {
    pub fn to_document(&self, zip_path: &Path, psd_path_in_zip: &Path) -> PsdDocument {
        PsdDocument {
            zip_path: zip_path.to_path_buf(),
            psd_path_in_zip: psd_path_in_zip.to_path_buf(),
            file_name: self.file_name.clone(),
            metadata: self.metadata.clone(),
            layers: self.layer_descriptors.clone(),
            error: self.error.clone(),
            log_path: self.log_path.clone(),
            default_rendered_png_path: self.rendered_png_path.clone(),
            render_warnings: self.render_warnings.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZipEntry {
    pub zip_path: PathBuf,
    pub zip_hash: String,
    pub cache_dir: PathBuf,
    pub source_zip_path: PathBuf,
    pub extracted_dir: PathBuf,
    pub psd_meta_path: PathBuf,
    pub psds: Vec<PsdEntry>,
    pub updated_at: u64,
}
