use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use rawpsd::{parse_layer_records, parse_psd_metadata, LayerInfo, PsdMetadata};

use crate::api::{LayerDescriptor, LayerVisibilityOverride};
use crate::layer_name_format::{is_mandatory_kind, is_mandatory_name};
use crate::logging::{clear_psd_failure_log, write_psd_failure_log, PsdFailureLog};
use crate::model::{LayerKind, LayerNode, PsdEntry};
use crate::render::render_png;

pub(crate) struct PsdAnalysis {
    pub(crate) file_name: String,
    pub(crate) metadata_label: String,
    pub(crate) layer_nodes: Vec<LayerNode>,
    pub(crate) layer_descriptors: Vec<LayerDescriptor>,
    pub(crate) effective_visibility: Vec<bool>,
    pub(crate) metadata: Option<PsdMetadata>,
    pub(crate) layers: Vec<LayerInfo>,
    pub(crate) diagnostics: Vec<String>,
    pub(crate) backtrace: Option<String>,
    pub(crate) data_len: usize,
    pub(crate) updated_at: u64,
}

impl PsdAnalysis {
    pub(crate) fn visible_error(&self) -> Option<String> {
        self.diagnostics.first().cloned()
    }

    pub(crate) fn can_render(&self) -> bool {
        self.visible_error().is_none() && self.metadata.is_some()
    }
}

pub(crate) fn build_psd_entry(path: &Path, render_root: &Path) -> PsdEntry {
    let mut analysis = analyze_psd(path);
    let mut rendered_png_path = None;
    let mut render_warnings = Vec::new();

    if let Some(metadata) = analysis.metadata.as_ref().filter(|_| analysis.can_render()) {
        let render_path = render_root.join(rendered_png_name(path));
        match render_png(
            metadata,
            &analysis.layers,
            &analysis.effective_visibility,
            &render_path,
        ) {
            Ok(render_result) => {
                rendered_png_path = Some(render_result.output_path);
                render_warnings = render_result.warnings;
            }
            Err(error) => analysis
                .diagnostics
                .push(format!("render_default_png error: {error}")),
        }
    }

    let visible_error = analysis.visible_error();
    let log_path = commit_diagnostics_log(
        path,
        &analysis.metadata_label,
        &analysis.diagnostics,
        analysis.data_len,
        analysis.backtrace.as_deref(),
    );

    PsdEntry {
        path: path.to_path_buf(),
        file_name: analysis.file_name,
        metadata: analysis.metadata_label,
        layer_nodes: analysis.layer_nodes,
        layer_descriptors: analysis.layer_descriptors,
        error: visible_error,
        log_path,
        rendered_png_path,
        render_warnings,
        updated_at: analysis.updated_at,
    }
}

pub(crate) fn analyze_psd(path: &Path) -> PsdAnalysis {
    let file_name = psd_file_name(path);
    let updated_at = unix_timestamp();
    let mut diagnostics = Vec::new();
    let mut backtrace = None;

    let data = match fs::read(path).with_context(|| format!("failed to read {}", path.display())) {
        Ok(data) => data,
        Err(error) => {
            diagnostics.push(format!("{error:#}"));
            return PsdAnalysis {
                file_name,
                metadata_label: "Metadata unavailable".to_string(),
                layer_nodes: Vec::new(),
                layer_descriptors: Vec::new(),
                effective_visibility: Vec::new(),
                metadata: None,
                layers: Vec::new(),
                diagnostics,
                backtrace: None,
                data_len: 0,
                updated_at,
            };
        }
    };

    let metadata = match catch_parser_panic("parse_psd_metadata", || parse_psd_metadata(&data)) {
        Ok(Ok(metadata)) => Some(metadata),
        Ok(Err(error)) => {
            diagnostics.push(format!("parse_psd_metadata error: {error}"));
            None
        }
        Err(error) => {
            diagnostics.push(error.message.clone());
            backtrace = Some(error.backtrace.clone());
            None
        }
    };

    let metadata_label = metadata
        .as_ref()
        .map(metadata_label)
        .unwrap_or_else(|| "Metadata unavailable".to_string());

    let layers = match catch_parser_panic("parse_layer_records", || parse_layer_records(&data)) {
        Ok(Ok(layers)) => layers,
        Ok(Err((layers, error))) => {
            diagnostics.push(format!("parse_layer_records error: {error}"));
            layers
        }
        Err(error) => {
            diagnostics.push(error.message.clone());
            backtrace = Some(error.backtrace.clone());
            Vec::new()
        }
    };

    let layer_view = build_layer_view(&layers, &HashMap::new());

    PsdAnalysis {
        file_name,
        metadata_label,
        layer_nodes: layer_view.nodes,
        layer_descriptors: layer_view.descriptors,
        effective_visibility: layer_view.effective_visibility,
        metadata,
        layers,
        diagnostics,
        backtrace,
        data_len: data.len(),
        updated_at,
    }
}

#[cfg(test)]
pub(crate) fn build_layer_nodes(layers: &[LayerInfo]) -> (Vec<LayerNode>, Vec<bool>) {
    let layer_view = build_layer_view(layers, &HashMap::new());
    (layer_view.nodes, layer_view.effective_visibility)
}

pub(crate) fn effective_visibility_with_overrides(
    layers: &[LayerInfo],
    overrides: &[LayerVisibilityOverride],
) -> Result<Vec<bool>> {
    let mut override_map = HashMap::new();

    for override_entry in overrides {
        if override_entry.layer_index >= layers.len() {
            bail!(
                "layer_index {} is out of range for {} layers",
                override_entry.layer_index,
                layers.len()
            );
        }
        override_map.insert(override_entry.layer_index, override_entry.visible);
    }

    Ok(build_layer_view(layers, &override_map).effective_visibility)
}

pub(crate) fn catch_parser_panic<T, F>(label: &str, parser: F) -> Result<T, ParserPanic>
where
    F: FnOnce() -> T,
{
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let result = panic::catch_unwind(AssertUnwindSafe(parser));

    panic::set_hook(hook);

    result.map_err(|payload| ParserPanic {
        message: format!("{label} panicked: {}", panic_payload_message(payload)),
        backtrace: Backtrace::force_capture().to_string(),
    })
}

pub(crate) fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

pub(crate) fn psd_file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn rendered_png_name(path: &Path) -> String {
    let mut parts = Vec::new();
    let mut cache_index = None;

    for component in path.components() {
        if let Component::Normal(part) = component {
            let value = part.to_string_lossy();
            if value == "cache" {
                cache_index = Some(parts.len());
            }
            let sanitized = sanitize_component(&value);
            if !sanitized.is_empty() {
                parts.push(sanitized);
            }
        }
    }

    if let Some(index) = cache_index {
        parts.drain(..=index);
    }

    let stem = if parts.is_empty() {
        "psd".to_string()
    } else {
        parts.join("__")
    };
    format!("{stem}.png")
}

pub(crate) fn display_layer_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "(unnamed)".to_string()
    } else {
        trimmed.to_string()
    }
}

fn metadata_label(metadata: &PsdMetadata) -> String {
    format!(
        "{}x{} {}ch depth {}",
        metadata.width, metadata.height, metadata.channel_count, metadata.depth
    )
}

fn commit_diagnostics_log(
    path: &Path,
    metadata: &str,
    diagnostics: &[String],
    data_len: usize,
    backtrace: Option<&str>,
) -> Option<PathBuf> {
    if diagnostics.is_empty() {
        clear_psd_failure_log(path);
        None
    } else {
        write_psd_failure_log(&PsdFailureLog {
            psd_path: path,
            metadata,
            details: diagnostics,
            data_len,
            backtrace,
        })
    }
}

fn sanitize_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

fn build_layer_view(layers: &[LayerInfo], overrides: &HashMap<usize, bool>) -> LayerView {
    let mut descriptors = Vec::with_capacity(layers.len());
    let mut nodes = Vec::with_capacity(layers.len());
    let mut effective_visibility = vec![false; layers.len()];
    let mut depth = 0usize;
    let mut group_visibility = Vec::new();

    for (original_index, layer) in layers.iter().enumerate().rev() {
        let raw_visible = if is_mandatory_raw_layer(layer) {
            true
        } else {
            overrides
                .get(&original_index)
                .copied()
                .unwrap_or(layer.is_visible)
        };

        if layer.group_closer {
            let inherited_visible = group_visibility.iter().all(|is_visible| *is_visible);
            let visible = raw_visible && inherited_visible;
            let descriptor = LayerDescriptor {
                layer_index: original_index,
                name: display_layer_name(&layer.name),
                kind: LayerKind::GroupClose,
                default_visible: raw_visible,
                effective_visible: visible,
                depth: depth.saturating_sub(1),
            };
            descriptors.push(descriptor.clone());
            nodes.push(LayerNode {
                name: descriptor.name,
                kind: descriptor.kind,
                visible,
                depth: descriptor.depth,
            });
            depth = depth.saturating_sub(1);
            group_visibility.pop();
            continue;
        }

        let inherited_visible = group_visibility.iter().all(|is_visible| *is_visible);
        let visible = raw_visible && inherited_visible;
        effective_visibility[original_index] = visible;

        let kind = if layer.group_opener {
            LayerKind::GroupOpen
        } else {
            LayerKind::Layer
        };
        let descriptor = LayerDescriptor {
            layer_index: original_index,
            name: display_layer_name(&layer.name),
            kind,
            default_visible: raw_visible,
            effective_visible: visible,
            depth,
        };
        descriptors.push(descriptor.clone());
        nodes.push(LayerNode {
            name: descriptor.name,
            kind: descriptor.kind,
            visible,
            depth: descriptor.depth,
        });

        if layer.group_opener {
            group_visibility.push(visible);
            depth += 1;
        }
    }

    LayerView {
        descriptors,
        nodes,
        effective_visibility,
    }
}

struct LayerView {
    descriptors: Vec<LayerDescriptor>,
    nodes: Vec<LayerNode>,
    effective_visibility: Vec<bool>,
}

fn is_mandatory_raw_layer(layer: &LayerInfo) -> bool {
    let kind = if layer.group_opener {
        crate::model::LayerKind::GroupOpen
    } else if layer.group_closer {
        crate::model::LayerKind::GroupClose
    } else {
        crate::model::LayerKind::Layer
    };

    is_mandatory_kind(kind) && is_mandatory_name(&display_layer_name(&layer.name))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParserPanic {
    pub(crate) message: String,
    pub(crate) backtrace: String,
}
