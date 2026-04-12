use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use mascot_render_control::log_server_performance_info;
use mascot_render_core::{
    auto_generate_mouth_flap_target, build_mouth_flap_display_diffs, load_variation_spec,
    variation_spec_path, Core, DisplayDiff, MascotConfig, PsdDocument, PsdInspectReport,
    RenderPngReport, RenderRequest, RenderedPng, ZipEntryLoadReport,
};

#[derive(Debug)]
pub(crate) struct MouthFlapPngs {
    pub(crate) open_png_path: PathBuf,
    pub(crate) closed_png_path: PathBuf,
}

struct MouthFlapFrame {
    output_path: PathBuf,
    render_cache_hit: bool,
    report: RenderPngReport,
}

pub(crate) fn render_mouth_flap_pngs(
    core: &Core,
    config: &MascotConfig,
    mut record_stage: impl FnMut(&'static str, u64),
) -> Result<Option<MouthFlapPngs>> {
    let total_started_at = Instant::now();
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

    let load_display_diff_started_at = Instant::now();
    let loaded_display_diff = load_current_display_diff(config);
    let load_display_diff_ms = elapsed_ms_since(load_display_diff_started_at);
    record_stage("mouth_flap.load_display_diff", load_display_diff_ms);
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=load_display_diff elapsed_ms={} source={} override_count={} source_path={} active_png_path={} zip_path={} psd_path_in_zip={}",
        load_display_diff_ms,
        loaded_display_diff.source,
        loaded_display_diff.display_diff.visibility_overrides.len(),
        optional_path_text(loaded_display_diff.source_path.as_deref()),
        config.png_path.display(),
        config.zip_path.display(),
        config.psd_path_in_zip.display(),
    ));

    let inspect_started_at = Instant::now();
    let (document, inspect_report) = core
        .inspect_psd_with_report(&config.zip_path, &config.psd_path_in_zip)
        .with_context(|| {
            format!(
                "failed to inspect PSD '{}' for mouth flap",
                config.psd_path_in_zip.display()
            )
        })?;
    let inspect_ms = elapsed_ms_since(inspect_started_at);
    record_stage("mouth_flap.inspect_psd", inspect_ms);
    record_zip_load_stages(
        "mouth_flap.inspect_psd",
        &inspect_report.zip_entry,
        &mut record_stage,
    );
    log_mouth_flap_inspect(config, &document, &inspect_report, inspect_ms);

    let find_target_started_at = Instant::now();
    let target = match auto_generate_mouth_flap_target(&document, &loaded_display_diff.display_diff)
    {
        Ok(target) => target,
        Err(error) => {
            let find_target_ms = elapsed_ms_since(find_target_started_at);
            record_stage("mouth_flap.find_target", find_target_ms);
            let total_ms = elapsed_ms_since(total_started_at);
            log_server_performance_info(format!(
                "event=mouth_flap_png_generation stage=skipped elapsed_ms={} find_target_ms={} psd_file={} layer_count={} reason={}",
                total_ms,
                find_target_ms,
                psd_file_name,
                document.layers.len(),
                compact_log_value(&error),
            ));
            return Ok(None);
        }
    };
    let find_target_ms = elapsed_ms_since(find_target_started_at);
    record_stage("mouth_flap.find_target", find_target_ms);
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=find_target elapsed_ms={} psd_file={} layer_count={} open_layers={} closed_layers={}",
        find_target_ms,
        psd_file_name,
        document.layers.len(),
        compact_log_value(&target.open_layer_names.join("|")),
        compact_log_value(&target.closed_layer_names.join("|")),
    ));

    let build_display_diffs_started_at = Instant::now();
    let display_diffs =
        build_mouth_flap_display_diffs(&document, &loaded_display_diff.display_diff, &target)
            .map_err(|error| anyhow!(error))
            .with_context(|| {
                format!(
                    "failed to build mouth flap variations for '{}'",
                    psd_file_name
                )
            })?;
    let build_display_diffs_ms = elapsed_ms_since(build_display_diffs_started_at);
    record_stage("mouth_flap.build_display_diffs", build_display_diffs_ms);
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=build_display_diffs elapsed_ms={} open_override_count={} closed_override_count={}",
        build_display_diffs_ms,
        display_diffs.open.visibility_overrides.len(),
        display_diffs.closed.visibility_overrides.len(),
    ));

    let open_frame = render_mouth_flap_frame(
        core,
        config,
        psd_file_name,
        "open",
        display_diffs.open,
        &mut record_stage,
    )?;
    let closed_frame = render_mouth_flap_frame(
        core,
        config,
        psd_file_name,
        "closed",
        display_diffs.closed,
        &mut record_stage,
    )?;
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=completed elapsed_ms={} any_zip_extracted={} any_psd_meta_rebuilt={} psd_entries_built={} variation_png_cache_miss_count={} open_render_cache_hit={} closed_render_cache_hit={} inspect_zip_memory_cache_hit={} inspect_zip_meta_cache_hit={} open_png_path={} closed_png_path={}",
        elapsed_ms_since(total_started_at),
        inspect_report.zip_entry.zip_extracted
            || open_frame.report.zip_entry.zip_extracted
            || closed_frame.report.zip_entry.zip_extracted,
        inspect_report.zip_entry.psd_meta_rebuilt
            || open_frame.report.zip_entry.psd_meta_rebuilt
            || closed_frame.report.zip_entry.psd_meta_rebuilt,
        inspect_report
            .zip_entry
            .psd_entries_built
            .saturating_add(open_frame.report.zip_entry.psd_entries_built)
            .saturating_add(closed_frame.report.zip_entry.psd_entries_built),
        usize::from(!open_frame.report.variation_cache_hit)
            + usize::from(!closed_frame.report.variation_cache_hit),
        open_frame.render_cache_hit,
        closed_frame.render_cache_hit,
        inspect_report.zip_entry.memory_cache_hit,
        inspect_report.zip_entry.meta_cache_hit,
        open_frame.output_path.display(),
        closed_frame.output_path.display(),
    ));
    Ok(Some(MouthFlapPngs {
        open_png_path: open_frame.output_path,
        closed_png_path: closed_frame.output_path,
    }))
}

fn render_mouth_flap_frame(
    core: &Core,
    config: &MascotConfig,
    psd_file_name: &str,
    frame: &'static str,
    display_diff: DisplayDiff,
    record_stage: &mut impl FnMut(&'static str, u64),
) -> Result<MouthFlapFrame> {
    let started_at = Instant::now();
    let (rendered, report) = core
        .render_png_with_report(RenderRequest {
            zip_path: config.zip_path.clone(),
            psd_path_in_zip: config.psd_path_in_zip.clone(),
            display_diff,
        })
        .with_context(|| format!("failed to render mouth flap PNG for '{}'", psd_file_name))?;
    let elapsed_ms = elapsed_ms_since(started_at);
    record_stage(render_stage_name(frame), elapsed_ms);
    record_render_report_stages(frame, &report, record_stage);
    log_mouth_flap_render_frame(frame, &rendered, &report, elapsed_ms);
    Ok(MouthFlapFrame {
        output_path: rendered.output_path,
        render_cache_hit: rendered.cache_hit,
        report,
    })
}

struct LoadedDisplayDiff {
    display_diff: DisplayDiff,
    source: &'static str,
    source_path: Option<PathBuf>,
}

fn load_current_display_diff(config: &MascotConfig) -> LoadedDisplayDiff {
    let active_variation_path = variation_spec_path(&config.png_path);
    if let Some(display_diff) = load_variation_spec(
        &active_variation_path,
        &config.zip_path,
        &config.psd_path_in_zip,
    ) {
        return LoadedDisplayDiff {
            display_diff,
            source: "active_variation_spec",
            source_path: Some(active_variation_path),
        };
    }

    if let Some((path, display_diff)) = config.display_diff_path.as_deref().and_then(|path| {
        load_variation_spec(path, &config.zip_path, &config.psd_path_in_zip)
            .map(|display_diff| (path, display_diff))
    }) {
        return LoadedDisplayDiff {
            display_diff,
            source: "configured_display_diff",
            source_path: Some(path.to_path_buf()),
        };
    }

    LoadedDisplayDiff {
        display_diff: DisplayDiff::default(),
        source: "default",
        source_path: None,
    }
}

fn render_stage_name(frame: &str) -> &'static str {
    match frame {
        "open" => "mouth_flap.render_open_png",
        "closed" => "mouth_flap.render_closed_png",
        _ => "mouth_flap.render_png",
    }
}

fn record_zip_load_stages(
    stage_prefix: &str,
    report: &ZipEntryLoadReport,
    record_stage: &mut impl FnMut(&'static str, u64),
) {
    match stage_prefix {
        "mouth_flap.inspect_psd" => {
            record_stage("mouth_flap.inspect_psd.zip_load", report.elapsed_ms);
            if report.zip_extracted {
                record_stage("mouth_flap.inspect_psd.zip_extract", report.extract_ms);
            }
            if report.psd_meta_rebuilt {
                record_stage(
                    "mouth_flap.inspect_psd.psd_meta_rebuild",
                    report.rebuild_meta_ms,
                );
                record_stage(
                    "mouth_flap.inspect_psd.psd_entry_build",
                    report.psd_entry_build_ms,
                );
            }
        }
        "mouth_flap.render_open_png" => {
            record_stage("mouth_flap.render_open_png.zip_load", report.elapsed_ms);
            if report.zip_extracted {
                record_stage("mouth_flap.render_open_png.zip_extract", report.extract_ms);
            }
            if report.psd_meta_rebuilt {
                record_stage(
                    "mouth_flap.render_open_png.psd_meta_rebuild",
                    report.rebuild_meta_ms,
                );
                record_stage(
                    "mouth_flap.render_open_png.psd_entry_build",
                    report.psd_entry_build_ms,
                );
            }
        }
        "mouth_flap.render_closed_png" => {
            record_stage("mouth_flap.render_closed_png.zip_load", report.elapsed_ms);
            if report.zip_extracted {
                record_stage(
                    "mouth_flap.render_closed_png.zip_extract",
                    report.extract_ms,
                );
            }
            if report.psd_meta_rebuilt {
                record_stage(
                    "mouth_flap.render_closed_png.psd_meta_rebuild",
                    report.rebuild_meta_ms,
                );
                record_stage(
                    "mouth_flap.render_closed_png.psd_entry_build",
                    report.psd_entry_build_ms,
                );
            }
        }
        _ => {}
    }
}

fn record_render_report_stages(
    frame: &str,
    report: &RenderPngReport,
    record_stage: &mut impl FnMut(&'static str, u64),
) {
    let stage_prefix = render_stage_name(frame);
    record_zip_load_stages(stage_prefix, &report.zip_entry, record_stage);
    match frame {
        "open" => {
            record_stage(
                "mouth_flap.render_open_png.save_variation_spec",
                report.save_variation_spec_ms,
            );
            record_stage(
                "mouth_flap.render_open_png.psd_analyze",
                report.custom_psd_analyze_ms,
            );
            record_stage(
                "mouth_flap.render_open_png.effective_visibility",
                report.effective_visibility_ms,
            );
            record_stage(
                "mouth_flap.render_open_png.compose_save_png",
                report.compose_and_save_png_ms,
            );
            record_stage(
                "mouth_flap.render_open_png.write_render_meta",
                report.write_render_meta_ms,
            );
        }
        "closed" => {
            record_stage(
                "mouth_flap.render_closed_png.save_variation_spec",
                report.save_variation_spec_ms,
            );
            record_stage(
                "mouth_flap.render_closed_png.psd_analyze",
                report.custom_psd_analyze_ms,
            );
            record_stage(
                "mouth_flap.render_closed_png.effective_visibility",
                report.effective_visibility_ms,
            );
            record_stage(
                "mouth_flap.render_closed_png.compose_save_png",
                report.compose_and_save_png_ms,
            );
            record_stage(
                "mouth_flap.render_closed_png.write_render_meta",
                report.write_render_meta_ms,
            );
        }
        _ => {}
    }
}

fn log_mouth_flap_inspect(
    config: &MascotConfig,
    document: &PsdDocument,
    report: &PsdInspectReport,
    elapsed_ms: u64,
) {
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=inspect_psd elapsed_ms={} document_elapsed_ms={} layer_count={} psd_error_present={} default_rendered_png_path={} log_path={} zip_load_ms={} zip_memory_cache_hit={} zip_meta_cache_hit={} zip_extracted={} psd_meta_rebuilt={} psd_entries_built={} zip_extract_ms={} psd_entry_build_ms={} rebuild_meta_ms={} zip_path={} psd_path_in_zip={}",
        elapsed_ms,
        report.elapsed_ms,
        document.layers.len(),
        document.error.is_some(),
        optional_path_text(document.default_rendered_png_path.as_deref()),
        optional_path_text(document.log_path.as_deref()),
        report.zip_entry.elapsed_ms,
        report.zip_entry.memory_cache_hit,
        report.zip_entry.meta_cache_hit,
        report.zip_entry.zip_extracted,
        report.zip_entry.psd_meta_rebuilt,
        report.zip_entry.psd_entries_built,
        report.zip_entry.extract_ms,
        report.zip_entry.psd_entry_build_ms,
        report.zip_entry.rebuild_meta_ms,
        config.zip_path.display(),
        config.psd_path_in_zip.display(),
    ));
}

fn log_mouth_flap_render_frame(
    frame: &str,
    rendered: &RenderedPng,
    report: &RenderPngReport,
    elapsed_ms: u64,
) {
    log_server_performance_info(format!(
        "event=mouth_flap_png_generation stage=render_{}_png elapsed_ms={} output_path={} render_cache_hit={} warning_count={} total_render_ms={} default_render={} variation_cache_hit={} save_variation_spec_ms={} custom_psd_analyze_ms={} effective_visibility_ms={} compose_and_save_png_ms={} write_render_meta_ms={} zip_load_ms={} zip_memory_cache_hit={} zip_meta_cache_hit={} zip_extracted={} psd_meta_rebuilt={} psd_entries_built={} zip_extract_ms={} psd_entry_build_ms={} rebuild_meta_ms={}",
        frame,
        elapsed_ms,
        rendered.output_path.display(),
        rendered.cache_hit,
        rendered.warnings.len(),
        report.elapsed_ms,
        report.default_render,
        report.variation_cache_hit,
        report.save_variation_spec_ms,
        report.custom_psd_analyze_ms,
        report.effective_visibility_ms,
        report.compose_and_save_png_ms,
        report.write_render_meta_ms,
        report.zip_entry.elapsed_ms,
        report.zip_entry.memory_cache_hit,
        report.zip_entry.meta_cache_hit,
        report.zip_entry.zip_extracted,
        report.zip_entry.psd_meta_rebuilt,
        report.zip_entry.psd_entries_built,
        report.zip_entry.extract_ms,
        report.zip_entry.psd_entry_build_ms,
        report.zip_entry.rebuild_meta_ms,
    ));
}

fn optional_path_text(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn compact_log_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("_")
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}
