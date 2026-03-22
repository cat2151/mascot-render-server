use std::path::{Path, PathBuf};

use mascot_render_core::display_path;

pub(super) fn current_preview_status(
    preview_png_path: Option<&Path>,
    variation_spec_path: Option<&Path>,
) -> String {
    match (preview_png_path, variation_spec_path) {
        (Some(preview), Some(spec)) => format!(
            "Variation cached: {} | Preview: {}",
            display_path(spec),
            display_path(preview)
        ),
        (Some(preview), None) => format!("Preview reset to default: {}", display_path(preview)),
        (None, Some(spec)) => format!(
            "Variation selected without preview PNG: {}",
            display_path(spec)
        ),
        (None, None) => "No cached PNG preview.".to_string(),
    }
}

pub(super) fn psd_path_in_zip(psd_path: &Path, extracted_dir: &Path, fallback: &Path) -> PathBuf {
    psd_path
        .strip_prefix(extracted_dir)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| fallback.to_path_buf())
}
