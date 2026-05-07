use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use mascot_render_control::{log_server_error, log_server_info};
use mascot_render_core::MascotConfig;

pub(crate) fn configure_ui_fonts(ctx: &egui::Context, config: &MascotConfig) {
    #[cfg(target_os = "windows")]
    {
        if let Err(error) = install_windows_japanese_font(ctx, &config.ui_font_paths) {
            log_server_error(format!(
                "failed to configure Windows Japanese UI font; context menu labels may be unreadable: {error:#}"
            ));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = ctx;
    }
}

#[cfg(target_os = "windows")]
fn install_windows_japanese_font(
    ctx: &egui::Context,
    configured_paths: &[PathBuf],
) -> anyhow::Result<()> {
    let (font_name, font_bytes, font_path) = load_windows_japanese_font(configured_paths)?;
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        font_name.clone(),
        Arc::new(FontData::from_owned(font_bytes)),
    );
    prepend_font_family(&mut fonts, FontFamily::Proportional, &font_name);
    prepend_font_family(&mut fonts, FontFamily::Monospace, &font_name);
    ctx.set_fonts(fonts);
    log_server_info(format!(
        "configured Windows Japanese UI font for egui context menus: {}",
        font_path.display()
    ));
    Ok(())
}

#[cfg(target_os = "windows")]
fn prepend_font_family(fonts: &mut FontDefinitions, family: FontFamily, font_name: &str) {
    let family_fonts = fonts.families.entry(family).or_default();
    if family_fonts.iter().all(|existing| existing != font_name) {
        family_fonts.insert(0, font_name.to_owned());
    }
}

#[cfg(target_os = "windows")]
fn load_windows_japanese_font(
    configured_paths: &[PathBuf],
) -> anyhow::Result<(String, Vec<u8>, PathBuf)> {
    let tried_paths = windows_japanese_font_candidates(configured_paths);
    for path in &tried_paths {
        if !path.is_file() {
            continue;
        }
        if let Ok(font_bytes) = std::fs::read(path) {
            return Ok((font_name_from_path(path), font_bytes, path.clone()));
        }
    }
    anyhow::bail!(
        "no Japanese-capable Windows font found in {}",
        tried_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[cfg(target_os = "windows")]
fn font_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("windows-japanese-font")
        .to_owned()
}

#[cfg(target_os = "windows")]
fn windows_japanese_font_candidates(configured_paths: &[PathBuf]) -> Vec<PathBuf> {
    if !configured_paths.is_empty() {
        return configured_paths.to_vec();
    }
    default_windows_japanese_font_candidates()
}

#[cfg(target_os = "windows")]
fn default_windows_japanese_font_candidates() -> Vec<PathBuf> {
    let windows_root = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    default_windows_japanese_font_candidates_for_root(&windows_root)
}

#[cfg(target_os = "windows")]
fn default_windows_japanese_font_candidates_for_root(windows_root: &Path) -> Vec<PathBuf> {
    let fonts_dir = windows_root.join("Fonts");
    ["YuGothR.ttc", "YuGothM.ttc", "meiryo.ttc", "msgothic.ttc"]
        .into_iter()
        .map(|file_name| fonts_dir.join(file_name))
        .collect()
}

#[cfg(all(test, target_os = "windows"))]
pub(crate) fn windows_japanese_font_candidates_for_test(
    configured_paths: &[PathBuf],
    windows_root: &Path,
) -> Vec<PathBuf> {
    if !configured_paths.is_empty() {
        configured_paths.to_vec()
    } else {
        default_windows_japanese_font_candidates_for_root(windows_root)
    }
}
