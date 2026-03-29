use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use mascot_render_core::{
    auto_generate_mouth_flap_target, build_mouth_flap_display_diffs, load_variation_spec,
    variation_spec_path, Core, DisplayDiff, MascotConfig, RenderRequest,
};

#[derive(Debug)]
pub(crate) struct MouthFlapPngs {
    pub(crate) open_png_path: PathBuf,
    pub(crate) closed_png_path: PathBuf,
}

pub(crate) fn render_mouth_flap_pngs(
    core: &Core,
    config: &MascotConfig,
) -> Result<Option<MouthFlapPngs>> {
    let psd_file_name = config
        .psd_path_in_zip
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            anyhow!(
                "invalid PSD file name in '{}'",
                config.psd_path_in_zip.display()
            )
        })?;
    let base_variation = load_current_display_diff(config);
    let document = core
        .inspect_psd(&config.zip_path, &config.psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect PSD '{}' for mouth flap",
                config.psd_path_in_zip.display()
            )
        })?;
    let target = match auto_generate_mouth_flap_target(&document, &base_variation) {
        Ok(target) => target,
        Err(error) => {
            eprintln!(
                "Mouth flap auto-generation skipped for '{}': {}",
                psd_file_name, error
            );
            return Ok(None);
        }
    };
    let display_diffs = build_mouth_flap_display_diffs(&document, &base_variation, &target)
        .map_err(|error| anyhow!(error))
        .with_context(|| {
            format!(
                "failed to build mouth flap variations for '{}'",
                psd_file_name
            )
        })?;

    Ok(Some(MouthFlapPngs {
        open_png_path: render_mouth_flap_frame(core, config, psd_file_name, display_diffs.open)?,
        closed_png_path: render_mouth_flap_frame(
            core,
            config,
            psd_file_name,
            display_diffs.closed,
        )?,
    }))
}

fn render_mouth_flap_frame(
    core: &Core,
    config: &MascotConfig,
    psd_file_name: &str,
    display_diff: DisplayDiff,
) -> Result<PathBuf> {
    let rendered = core
        .render_png(RenderRequest {
            zip_path: config.zip_path.clone(),
            psd_path_in_zip: config.psd_path_in_zip.clone(),
            display_diff,
        })
        .with_context(|| format!("failed to render mouth flap PNG for '{}'", psd_file_name))?;
    Ok(rendered.output_path)
}

fn load_current_display_diff(config: &MascotConfig) -> DisplayDiff {
    let active_variation_path = variation_spec_path(&config.png_path);
    load_variation_spec(
        &active_variation_path,
        &config.zip_path,
        &config.psd_path_in_zip,
    )
    .or_else(|| {
        config
            .display_diff_path
            .as_deref()
            .and_then(|path| load_variation_spec(path, &config.zip_path, &config.psd_path_in_zip))
    })
    .unwrap_or_default()
}
