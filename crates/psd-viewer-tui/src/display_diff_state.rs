use mascot_render_core::{
    is_exclusive_kind, is_exclusive_name, is_mandatory_kind, is_mandatory_name, is_toggleable_kind,
    DisplayDiff, LayerDescriptor, LayerKind, LayerVisibilityOverride, PsdDocument,
};

#[derive(Debug, Clone)]
pub(crate) struct LayerRow {
    pub(crate) name: String,
    pub(crate) kind: LayerKind,
    pub(crate) visible: bool,
    pub(crate) depth: usize,
}

impl LayerRow {
    pub(crate) fn display_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let visibility = if self.visible { "visible" } else { "hidden" };
        format!(
            "{}{} {} ({})",
            indent,
            self.kind.tag(),
            self.name,
            visibility
        )
    }
}

pub(crate) fn resolve_layer_rows(
    document: &PsdDocument,
    display_diff: &DisplayDiff,
) -> Vec<LayerRow> {
    let mut rows = Vec::with_capacity(document.layers.len());
    let mut group_visibility = Vec::new();

    for descriptor in &document.layers {
        let raw_visible = resolved_raw_visibility(display_diff, descriptor);
        let inherited_visible = group_visibility.iter().all(|is_visible| *is_visible);
        let visible = raw_visible && inherited_visible;

        rows.push(LayerRow {
            name: descriptor.name.clone(),
            kind: descriptor.kind,
            visible,
            depth: descriptor.depth,
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

pub(crate) fn toggle_layer_override(
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

#[cfg(test)]
pub(crate) fn find_named_exclusive_pair(
    document: &PsdDocument,
    first_name: &str,
    second_name: &str,
) -> Option<(usize, usize)> {
    let mut best_match = None;

    for (first_index, first_descriptor) in document.layers.iter().enumerate() {
        if !is_named_exclusive_descriptor(first_descriptor, first_name) {
            continue;
        }

        for (second_index, second_descriptor) in document.layers.iter().enumerate() {
            if first_index == second_index
                || !is_named_exclusive_descriptor(second_descriptor, second_name)
                || first_descriptor.depth != second_descriptor.depth
                || exclusive_scope_bounds(document, first_index)
                    != exclusive_scope_bounds(document, second_index)
            {
                continue;
            }

            let distance = first_index.abs_diff(second_index);
            match best_match {
                Some((_, _, best_distance)) if best_distance <= distance => {}
                _ => best_match = Some((first_index, second_index, distance)),
            }
        }
    }

    best_match.map(|(first_index, second_index, _)| (first_index, second_index))
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

fn resolved_raw_visibility(display_diff: &DisplayDiff, descriptor: &LayerDescriptor) -> bool {
    if is_mandatory_descriptor(descriptor) {
        return true;
    }

    override_map(display_diff)
        .iter()
        .find(|entry| entry.layer_index == descriptor.layer_index)
        .map(|entry| entry.visible)
        .unwrap_or(descriptor.default_visible)
}

fn is_mandatory_descriptor(descriptor: &LayerDescriptor) -> bool {
    is_mandatory_kind(descriptor.kind) && is_mandatory_name(&descriptor.name)
}

fn is_exclusive_descriptor(descriptor: &LayerDescriptor) -> bool {
    is_exclusive_kind(descriptor.kind) && is_exclusive_name(&descriptor.name)
}

#[cfg(test)]
fn is_named_exclusive_descriptor(descriptor: &LayerDescriptor, name: &str) -> bool {
    is_exclusive_descriptor(descriptor) && normalized_layer_name(&descriptor.name) == name
}

#[cfg(test)]
fn normalized_layer_name(name: &str) -> &str {
    name.trim_start_matches(['*', '!'])
}

fn exclusive_sibling_layer_indices(document: &PsdDocument, row_index: usize) -> Vec<usize> {
    let Some(selected) = document.layers.get(row_index) else {
        return Vec::new();
    };
    let (scope_start, scope_end) = exclusive_scope_bounds(document, row_index);

    document.layers[scope_start..scope_end]
        .iter()
        .filter(|descriptor| {
            descriptor.layer_index != selected.layer_index
                && descriptor.depth == selected.depth
                && is_exclusive_descriptor(descriptor)
        })
        .map(|descriptor| descriptor.layer_index)
        .collect()
}

fn exclusive_scope_bounds(document: &PsdDocument, row_index: usize) -> (usize, usize) {
    let Some(group_open_index) = containing_group_open_index(document, row_index) else {
        return (0, document.layers.len());
    };
    let Some(group_close_index) = matching_group_close_index(document, group_open_index) else {
        return (group_open_index + 1, document.layers.len());
    };
    (group_open_index + 1, group_close_index)
}

fn containing_group_open_index(document: &PsdDocument, row_index: usize) -> Option<usize> {
    let mut nested_groups = 0usize;

    for index in (0..row_index).rev() {
        match document.layers[index].kind {
            LayerKind::GroupClose => nested_groups += 1,
            LayerKind::GroupOpen => {
                if nested_groups == 0 {
                    return Some(index);
                }
                nested_groups -= 1;
            }
            LayerKind::Layer => {}
        }
    }

    None
}

fn matching_group_close_index(document: &PsdDocument, group_open_index: usize) -> Option<usize> {
    let mut nested_groups = 0usize;

    for index in group_open_index + 1..document.layers.len() {
        match document.layers[index].kind {
            LayerKind::GroupOpen => nested_groups += 1,
            LayerKind::GroupClose => {
                if nested_groups == 0 {
                    return Some(index);
                }
                nested_groups -= 1;
            }
            LayerKind::Layer => {}
        }
    }

    None
}

fn override_map(display_diff: &DisplayDiff) -> &[LayerVisibilityOverride] {
    &display_diff.visibility_overrides
}

#[cfg(test)]
pub(crate) fn descriptor(
    layer_index: usize,
    name: &str,
    kind: LayerKind,
    default_visible: bool,
    effective_visible: bool,
    depth: usize,
) -> LayerDescriptor {
    LayerDescriptor {
        layer_index,
        name: name.to_string(),
        kind,
        default_visible,
        effective_visible,
        depth,
    }
}
