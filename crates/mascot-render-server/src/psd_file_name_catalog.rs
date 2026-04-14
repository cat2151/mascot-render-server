use std::sync::Arc;

use anyhow::Result;
use mascot_render_core::{Core, PsdEntry, ZipEntry};

/// Captures the usable PSD file names once during server startup.
/// The catalog never refreshes itself after construction.
#[derive(Debug, Clone)]
pub(crate) struct PsdFileNameCatalog {
    file_names: Arc<[String]>,
}

impl PsdFileNameCatalog {
    pub(crate) fn load_startup_fixed(core: &Core) -> Result<Self> {
        let entries = core.load_cached_zip_entries_snapshot()?;
        Ok(Self::from_entries(entries))
    }

    pub(crate) fn snapshot(&self) -> Vec<String> {
        self.file_names.iter().cloned().collect()
    }

    fn from_entries(entries: Vec<ZipEntry>) -> Self {
        let mut file_names = entries
            .iter()
            .flat_map(|entry| entry.psds.iter())
            .filter_map(usable_psd_file_name)
            .collect::<Vec<_>>();
        file_names.sort_unstable();
        file_names.dedup();
        Self {
            file_names: Arc::from(file_names),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_entries_for_test(entries: Vec<ZipEntry>) -> Self {
        Self::from_entries(entries)
    }
}

fn usable_psd_file_name(psd: &PsdEntry) -> Option<String> {
    (psd.rendered_png_path.is_some() && !psd.file_name.is_empty()).then(|| psd.file_name.clone())
}
