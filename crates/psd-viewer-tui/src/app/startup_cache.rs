use mascot_render_control::log_psd_viewer_tui_info;
use mascot_render_core::{rgba_cache_exists, skin_details_cache_exists, Core, ZipEntry};

pub(super) fn log_default_png_ready(core: &Core, zip_entries: &[ZipEntry]) {
    let psd_count = zip_entries.iter().map(|zip| zip.psds.len()).sum::<usize>();
    let default_png_ready_count = zip_entries
        .iter()
        .flat_map(|zip| zip.psds.iter())
        .filter(|psd| {
            psd.error.is_none()
                && psd
                    .rendered_png_path
                    .as_ref()
                    .is_some_and(|path| path.exists())
        })
        .count();
    let default_skin_details_ready_count = zip_entries
        .iter()
        .flat_map(|zip| zip.psds.iter())
        .filter(|psd| {
            psd.error.is_none()
                && psd
                    .rendered_png_path
                    .as_ref()
                    .is_some_and(|path| path.exists() && skin_details_cache_exists(path))
        })
        .count();
    let default_raw_rgba_ready_count = zip_entries
        .iter()
        .flat_map(|zip| zip.psds.iter())
        .filter(|psd| {
            psd.error.is_none()
                && psd
                    .rendered_png_path
                    .as_ref()
                    .is_some_and(|path| path.exists() && rgba_cache_exists(path))
        })
        .count();
    let render_unavailable_count = zip_entries
        .iter()
        .flat_map(|zip| zip.psds.iter())
        .filter(|psd| psd.error.is_some() || psd.rendered_png_path.is_none())
        .count();
    log_psd_viewer_tui_info(format!(
        "trigger=startup_loader action=default_png_ready zip_count={} psd_count={} default_png_ready_count={} default_skin_details_ready_count={} default_raw_rgba_ready_count={} render_unavailable_count={} cache_dir={}",
        zip_entries.len(),
        psd_count,
        default_png_ready_count,
        default_skin_details_ready_count,
        default_raw_rgba_ready_count,
        render_unavailable_count,
        core.cache_dir().display()
    ));
}
