use crate::api::PsdDocument;
use crate::layer_name_format::{is_exclusive_kind, is_exclusive_name};
use crate::model::LayerKind;

use super::{
    matching_group_close_index, normalized_layer_name, RowVisibilityState, MOUTH_GROUP_LAYER,
};

pub(super) fn format_missing_pair_diagnostics(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    open_layer_names: &[impl AsRef<str>],
    closed_layer_names: &[impl AsRef<str>],
) -> String {
    let groups =
        collect_mouth_group_diagnostics(document, states, open_layer_names, closed_layer_names);
    if groups.is_empty() {
        return format!(
            "No groups containing '{}' were found.\nopen candidates: [{}]\nclosed candidates: [{}]",
            MOUTH_GROUP_LAYER,
            format_name_list(open_layer_names),
            format_name_list(closed_layer_names)
        );
    }

    groups.join("\n")
}

fn collect_mouth_group_diagnostics(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    open_layer_names: &[impl AsRef<str>],
    closed_layer_names: &[impl AsRef<str>],
) -> Vec<String> {
    document
        .layers
        .iter()
        .enumerate()
        .filter_map(|(group_open_index, descriptor)| {
            if descriptor.kind != LayerKind::GroupOpen
                || !normalized_layer_name(&descriptor.name).contains(MOUTH_GROUP_LAYER)
            {
                return None;
            }

            Some(format_mouth_group_diagnostic(
                document,
                states,
                open_layer_names,
                closed_layer_names,
                group_open_index,
            ))
        })
        .collect()
}

fn format_mouth_group_diagnostic(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    open_layer_names: &[impl AsRef<str>],
    closed_layer_names: &[impl AsRef<str>],
    group_open_index: usize,
) -> String {
    let group = &document.layers[group_open_index];
    let layer_names = direct_exclusive_layer_names_in_group(document, group_open_index);
    let missing_open = !contains_any_name(&layer_names, open_layer_names);
    let missing_closed = !contains_any_name(&layer_names, closed_layer_names);
    let visible = states
        .get(group_open_index)
        .is_some_and(|state| state.visible);
    let mut lines = vec![format!(
        "- group '{}' [{}]",
        group.name,
        if visible { "visible" } else { "hidden" }
    )];

    if missing_open {
        lines.push(format!(
            "  open candidates [{}] were not found. layers: {}",
            format_name_list(open_layer_names),
            format_layer_name_list(&layer_names)
        ));
    }
    if missing_closed {
        lines.push(format!(
            "  closed candidates [{}] were not found. layers: {}",
            format_name_list(closed_layer_names),
            format_layer_name_list(&layer_names)
        ));
    }
    if !missing_open && !missing_closed && !visible {
        lines.push("  open/closed candidates exist, but this group is hidden.".to_string());
    }

    lines.join("\n")
}

fn direct_exclusive_layer_names_in_group(
    document: &PsdDocument,
    group_open_index: usize,
) -> Vec<String> {
    let group_open = &document.layers[group_open_index];
    let Some(group_close_index) = matching_group_close_index(document, group_open_index) else {
        return Vec::new();
    };

    let mut layer_names = Vec::new();
    for descriptor in &document.layers[group_open_index + 1..group_close_index] {
        if descriptor.depth != group_open.depth + 1
            || !is_exclusive_kind(descriptor.kind)
            || !is_exclusive_name(&descriptor.name)
        {
            continue;
        }

        let name = normalized_layer_name(&descriptor.name).to_string();
        if !layer_names.contains(&name) {
            layer_names.push(name);
        }
    }
    layer_names
}

fn contains_any_name(layer_names: &[String], candidates: &[impl AsRef<str>]) -> bool {
    candidates
        .iter()
        .any(|candidate| layer_names.iter().any(|name| name == candidate.as_ref()))
}

fn format_layer_name_list(layer_names: &[String]) -> String {
    if layer_names.is_empty() {
        "(none)".to_string()
    } else {
        layer_names.join(", ")
    }
}

pub(super) fn format_name_list(values: &[impl AsRef<str>]) -> String {
    values
        .iter()
        .map(|value| value.as_ref())
        .collect::<Vec<_>>()
        .join(", ")
}
