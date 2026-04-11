use std::path::PathBuf;

use crate::model::{PsdEntry, ZipEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipLoadProgress {
    pub zip_path: PathBuf,
    pub zip_cache_key: String,
    pub cache_dir: PathBuf,
    pub extracted_dir: PathBuf,
    pub psd_meta_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsdLoadProgress {
    pub zip: ZipLoadProgress,
    pub psd_path: PathBuf,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub enum ZipLoadEvent {
    ZipStarted(ZipLoadProgress),
    ZipExtracted(ZipLoadProgress),
    PsdDiscovered(PsdLoadProgress),
    PsdReady(PsdLoadProgress, Box<PsdEntry>),
    ZipReady(ZipEntry),
    Finished(Vec<ZipEntry>),
}
