use super::*;
use crate::api::LayerVisibilityOverride;
use crate::layer_name_format::is_toggleable_kind;

pub(super) fn row_label(document: &PsdDocument, row_index: usize) -> String {
    document
        .layers
        .get(row_index)
        .map(|descriptor| normalized_layer_name(&descriptor.name).to_string())
        .unwrap_or_default()
}

pub(super) fn ensure_named_row_visible(
    base_variation: &DisplayDiff,
    document: &PsdDocument,
    row_index: usize,
    label: &str,
) -> Result<DisplayDiff, String> {
    let mut variation = base_variation.clone();
    let rows = resolve_row_states(document, &variation);
    if rows.get(row_index).is_none() {
        return Err(format!("mouth flap target '{}' row is missing", label));
    }
    if rows[row_index].visible {
        return Ok(variation);
    }

    if !toggle_layer_override(&mut variation, document, row_index) {
        return Err(format!("failed to activate mouth flap target '{}'", label));
    }

    let rows = resolve_row_states(document, &variation);
    if rows.get(row_index).is_some_and(|state| state.visible) {
        return Ok(variation);
    }

    Err(format!(
        "mouth flap target '{}' stayed hidden after toggle; parent group may be hidden",
        label
    ))
}

pub(super) fn resolve_row_states(
    document: &PsdDocument,
    display_diff: &DisplayDiff,
) -> Vec<RowVisibilityState> {
    let mut rows = Vec::with_capacity(document.layers.len());
    let mut group_visibility = Vec::new();

    for descriptor in &document.layers {
        let raw_visible = resolved_raw_visibility(display_diff, descriptor);
        let parent_visible = group_visibility.iter().all(|is_visible| *is_visible);
        let visible = raw_visible && parent_visible;
        rows.push(RowVisibilityState {
            visible,
            parent_visible,
        });

        match descriptor.kind {
            LayerKind::GroupOpen => group_visibility.push(visible),
            LayerKind::GroupClose => {
                group_visibility.pop();
            }
            LayerKind::Layer => {}
        }
    }

    rows
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RowVisibilityState {
    pub(super) visible: bool,
    pub(super) parent_visible: bool,
}

fn resolved_raw_visibility(display_diff: &DisplayDiff, descriptor: &LayerDescriptor) -> bool {
    if is_mandatory_descriptor(descriptor) {
        return true;
    }

    display_diff
        .visibility_overrides
        .iter()
        .find(|entry| entry.layer_index == descriptor.layer_index)
        .map(|entry| entry.visible)
        .unwrap_or(descriptor.default_visible)
}

fn toggle_layer_override(
    display_diff: &mut DisplayDiff,
    document: &PsdDocument,
    row_index: usize,
) -> bool {
    let Some(descriptor) = document.layers.get(row_index) else {
        return false;
    };
    if !is_toggleable_kind(descriptor.kind) {
        return false;
    }

    let current_visible = resolved_raw_visibility(display_diff, descriptor);
    let next_visible = !current_visible;
    if current_visible
        && (is_mandatory_descriptor(descriptor) || is_exclusive_descriptor(descriptor))
    {
        return false;
    }

    set_layer_visibility(display_diff, descriptor.layer_index, next_visible);
    if next_visible && is_exclusive_descriptor(descriptor) {
        for sibling_layer_index in exclusive_sibling_layer_indices(document, row_index) {
            set_layer_visibility(display_diff, sibling_layer_index, false);
        }
    }

    normalize_overrides(display_diff, document);
    true
}

fn set_layer_visibility(display_diff: &mut DisplayDiff, layer_index: usize, visible: bool) {
    if let Some(entry) = display_diff
        .visibility_overrides
        .iter_mut()
        .find(|entry| entry.layer_index == layer_index)
    {
        entry.visible = visible;
    } else {
        display_diff
            .visibility_overrides
            .push(LayerVisibilityOverride {
                layer_index,
                visible,
            });
    }
}

fn normalize_overrides(display_diff: &mut DisplayDiff, document: &PsdDocument) {
    display_diff.visibility_overrides.retain(|entry| {
        match document
            .layers
            .iter()
            .find(|layer| layer.layer_index == entry.layer_index)
        {
            Some(layer) => entry.visible != layer.default_visible,
            None => true,
        }
    });
    display_diff
        .visibility_overrides
        .sort_by_key(|entry| entry.layer_index);
}

fn exclusive_sibling_layer_indices(document: &PsdDocument, row_index: usize) -> Vec<usize> {
    let Some(descriptor) = document.layers.get(row_index) else {
        return Vec::new();
    };
    let (scope_start, scope_end) = exclusive_scope_bounds(document, row_index);
    (scope_start..scope_end)
        .filter_map(|index| {
            let sibling = document.layers.get(index)?;
            (index != row_index
                && sibling.depth == descriptor.depth
                && is_exclusive_descriptor(sibling))
            .then_some(sibling.layer_index)
        })
        .collect()
}

fn is_exclusive_descriptor(descriptor: &LayerDescriptor) -> bool {
    is_exclusive_kind(descriptor.kind) && is_exclusive_name(&descriptor.name)
}
