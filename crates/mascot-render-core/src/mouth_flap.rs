use serde::{Deserialize, Serialize};

use crate::api::{DisplayDiff, LayerDescriptor, PsdDocument};
use crate::layer_name_format::{
    is_exclusive_kind, is_exclusive_name, is_mandatory_kind, is_mandatory_name,
};
use crate::model::LayerKind;

#[path = "mouth_flap/diagnostics.rs"]
mod diagnostics;
#[path = "mouth_flap/visibility.rs"]
mod visibility;

use diagnostics::{format_missing_pair_diagnostics, format_name_list};
use visibility::{ensure_named_row_visible, resolve_row_states, row_label, RowVisibilityState};

pub const MOUTH_GROUP_LAYER: &str = "口";
pub const MOUTH_OPEN_LAYER: &str = "ほあー";
pub const MOUTH_OPEN_LAYER_ALT_1: &str = "ほー";
pub const MOUTH_OPEN_LAYER_ALT_2: &str = "おー";
pub const MOUTH_OPEN_LAYER_ALT_3: &str = "お";
pub const MOUTH_CLOSED_LAYER: &str = "むふ";
pub const MOUTH_CLOSED_LAYER_ALT_1: &str = "むん";
pub const MOUTH_CLOSED_LAYER_ALT_2: &str = "んむ";
pub const MOUTH_CLOSED_LAYER_ALT_3: &str = "ん";
pub const MOUTH_CLOSED_LAYER_ALT_4: &str = "んー";
pub const DEFAULT_MOUTH_OPEN_LAYER_NAMES: &[&str] = &[
    MOUTH_OPEN_LAYER,
    MOUTH_OPEN_LAYER_ALT_1,
    MOUTH_OPEN_LAYER_ALT_2,
    MOUTH_OPEN_LAYER_ALT_3,
];
pub const DEFAULT_MOUTH_CLOSED_LAYER_NAMES: &[&str] = &[
    MOUTH_CLOSED_LAYER,
    MOUTH_CLOSED_LAYER_ALT_1,
    MOUTH_CLOSED_LAYER_ALT_2,
    MOUTH_CLOSED_LAYER_ALT_3,
    MOUTH_CLOSED_LAYER_ALT_4,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouthFlapTarget {
    pub psd_file_name: String,
    pub open_layer_names: Vec<String>,
    pub closed_layer_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouthFlapRows {
    pub open_row_index: usize,
    pub closed_row_index: usize,
    pub open_label: String,
    pub closed_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouthFlapDisplayDiffs {
    pub open: DisplayDiff,
    pub closed: DisplayDiff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenCandidate {
    row_index: usize,
    priority: usize,
    distance: usize,
    visible: bool,
}

impl OpenCandidate {
    fn is_better_than(&self, current: Option<Self>) -> bool {
        let Some(current) = current else {
            return true;
        };

        self.priority < current.priority
            || (self.priority == current.priority && self.visible && !current.visible)
            || (self.priority == current.priority
                && self.visible == current.visible
                && self.distance < current.distance)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PairCandidate {
    open_row_index: usize,
    closed_row_index: usize,
    closed_priority: usize,
    open_priority: usize,
    open_visible: bool,
    distance: usize,
}

impl PairCandidate {
    fn is_better_than(&self, current: Option<Self>) -> bool {
        let Some(current) = current else {
            return true;
        };

        self.closed_priority < current.closed_priority
            || (self.closed_priority == current.closed_priority
                && self.open_priority < current.open_priority)
            || (self.closed_priority == current.closed_priority
                && self.open_priority == current.open_priority
                && self.open_visible
                && !current.open_visible)
            || (self.closed_priority == current.closed_priority
                && self.open_priority == current.open_priority
                && self.open_visible == current.open_visible
                && self.distance < current.distance)
    }
}

pub fn auto_generate_mouth_flap_target(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
) -> Result<MouthFlapTarget, String> {
    auto_generate_mouth_flap_target_with_layer_name_slices(
        document,
        base_variation,
        DEFAULT_MOUTH_OPEN_LAYER_NAMES,
        DEFAULT_MOUTH_CLOSED_LAYER_NAMES,
    )
}

pub fn auto_generate_mouth_flap_target_with_layer_names<
    OpenLayerName: AsRef<str>,
    ClosedLayerName: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> Result<MouthFlapTarget, String> {
    auto_generate_mouth_flap_target_with_layer_name_slices(
        document,
        base_variation,
        open_layer_names,
        closed_layer_names,
    )
}

fn auto_generate_mouth_flap_target_with_layer_name_slices<
    OpenLayerName: AsRef<str>,
    ClosedLayerName: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> Result<MouthFlapTarget, String> {
    let states = resolve_row_states(document, base_variation);
    let (open_row_index, closed_row_index) =
        find_named_pair_in_visible_group(
            document,
            &states,
            MOUTH_GROUP_LAYER,
            open_layer_names,
            closed_layer_names,
        )
            .ok_or_else(|| {
                format!(
                    "PSD '{}' does not contain an auto-detectable mouth flap pair in visible '{}' groups matching open layers [{}] and closed layers [{}]",
                    document.file_name,
                    MOUTH_GROUP_LAYER,
                    format_name_list(open_layer_names),
                    format_name_list(closed_layer_names),
                )
            })?;

    Ok(MouthFlapTarget {
        psd_file_name: document.file_name.clone(),
        open_layer_names: vec![row_label(document, open_row_index)],
        closed_layer_names: vec![row_label(document, closed_row_index)],
    })
}

pub fn describe_mouth_flap_auto_generation_failure(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
) -> String {
    describe_mouth_flap_auto_generation_failure_with_layer_name_slices(
        document,
        base_variation,
        DEFAULT_MOUTH_OPEN_LAYER_NAMES,
        DEFAULT_MOUTH_CLOSED_LAYER_NAMES,
    )
}

pub fn describe_mouth_flap_auto_generation_failure_with_layer_names<
    OpenLayerName: AsRef<str>,
    ClosedLayerName: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> String {
    describe_mouth_flap_auto_generation_failure_with_layer_name_slices(
        document,
        base_variation,
        open_layer_names,
        closed_layer_names,
    )
}

fn describe_mouth_flap_auto_generation_failure_with_layer_name_slices<
    OpenLayerName: AsRef<str>,
    ClosedLayerName: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> String {
    let states = resolve_row_states(document, base_variation);
    format_missing_pair_diagnostics(document, &states, open_layer_names, closed_layer_names)
}

pub fn resolve_mouth_flap_rows(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    target: &MouthFlapTarget,
) -> Result<MouthFlapRows, String> {
    let states = resolve_row_states(document, base_variation);
    let (open_row_index, closed_row_index) = find_named_pair_in_visible_group(
        document,
        &states,
        MOUTH_GROUP_LAYER,
        &target.open_layer_names,
        &target.closed_layer_names,
    )
    .ok_or_else(|| {
        format!(
            "PSD does not contain a visible '{}' group with open layers [{}] and closed layers [{}]",
            MOUTH_GROUP_LAYER,
            target.open_layer_names.join(", "),
            target.closed_layer_names.join(", ")
        )
    })?;

    Ok(MouthFlapRows {
        open_row_index,
        closed_row_index,
        open_label: row_label(document, open_row_index),
        closed_label: row_label(document, closed_row_index),
    })
}

pub fn build_mouth_flap_display_diffs(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    target: &MouthFlapTarget,
) -> Result<MouthFlapDisplayDiffs, String> {
    let resolved = resolve_mouth_flap_rows(document, base_variation, target)?;
    Ok(MouthFlapDisplayDiffs {
        open: ensure_named_row_visible(
            base_variation,
            document,
            resolved.open_row_index,
            &resolved.open_label,
        )?,
        closed: ensure_named_row_visible(
            base_variation,
            document,
            resolved.closed_row_index,
            &resolved.closed_label,
        )?,
    })
}

fn find_named_pair_in_visible_group<OpenLayerName: AsRef<str>, ClosedLayerName: AsRef<str>>(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    group_name: &str,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> Option<(usize, usize)> {
    for (group_open_index, descriptor) in document.layers.iter().enumerate() {
        if descriptor.kind != LayerKind::GroupOpen
            || normalized_layer_name(&descriptor.name) != group_name
            || !states
                .get(group_open_index)
                .is_some_and(|state| state.visible)
        {
            continue;
        }

        let Some(pair) = find_named_pair_in_group_scope(
            document,
            states,
            group_open_index,
            open_layer_names,
            closed_layer_names,
        ) else {
            continue;
        };
        return Some(pair);
    }

    None
}

fn find_named_pair_in_group_scope<OpenLayerName: AsRef<str>, ClosedLayerName: AsRef<str>>(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    group_open_index: usize,
    open_layer_names: &[OpenLayerName],
    closed_layer_names: &[ClosedLayerName],
) -> Option<(usize, usize)> {
    let group_close_index =
        matching_group_close_index(document, group_open_index).unwrap_or(document.layers.len());
    let mut best_match = None::<PairCandidate>;

    for (closed_priority, closed_name) in closed_layer_names.iter().enumerate() {
        for closed_row_index in group_open_index + 1..group_close_index {
            let Some(closed_descriptor) = document.layers.get(closed_row_index) else {
                continue;
            };
            if !is_named_exclusive_descriptor(closed_descriptor, closed_name.as_ref())
                || !states
                    .get(closed_row_index)
                    .is_some_and(|state| state.parent_visible)
            {
                continue;
            }

            let Some(open_candidate) =
                find_open_row_in_scope(document, states, open_layer_names, closed_row_index)
            else {
                continue;
            };

            let candidate = PairCandidate {
                open_row_index: open_candidate.row_index,
                closed_row_index,
                closed_priority,
                open_priority: open_candidate.priority,
                open_visible: open_candidate.visible,
                distance: open_candidate.distance,
            };
            if candidate.is_better_than(best_match) {
                best_match = Some(candidate);
            }
        }
    }

    best_match.map(|candidate| (candidate.open_row_index, candidate.closed_row_index))
}

fn find_open_row_in_scope<OpenLayerName: AsRef<str>>(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    open_layer_names: &[OpenLayerName],
    closed_row_index: usize,
) -> Option<OpenCandidate> {
    let closed_descriptor = document.layers.get(closed_row_index)?;
    let (scope_start, scope_end) = exclusive_scope_bounds(document, closed_row_index);
    let mut best_match = None::<OpenCandidate>;

    for (open_priority, open_name) in open_layer_names.iter().enumerate() {
        for open_row_index in scope_start..scope_end {
            if open_row_index == closed_row_index {
                continue;
            }

            let descriptor = &document.layers[open_row_index];
            if descriptor.depth != closed_descriptor.depth
                || !is_named_exclusive_descriptor(descriptor, open_name.as_ref())
            {
                continue;
            }

            let open_visible = states
                .get(open_row_index)
                .is_some_and(|state| state.visible);
            let distance = open_row_index.abs_diff(closed_row_index);
            let candidate = OpenCandidate {
                row_index: open_row_index,
                priority: open_priority,
                distance,
                visible: open_visible,
            };
            if candidate.is_better_than(best_match) {
                best_match = Some(candidate);
            }
        }
    }

    best_match
}

fn is_mandatory_descriptor(descriptor: &LayerDescriptor) -> bool {
    is_mandatory_kind(descriptor.kind) && is_mandatory_name(&descriptor.name)
}

fn is_named_exclusive_descriptor(descriptor: &LayerDescriptor, name: &str) -> bool {
    is_exclusive_kind(descriptor.kind)
        && is_exclusive_name(&descriptor.name)
        && normalized_layer_name(&descriptor.name) == name
}

fn normalized_layer_name(name: &str) -> &str {
    name.trim_start_matches(['*', '!'])
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
