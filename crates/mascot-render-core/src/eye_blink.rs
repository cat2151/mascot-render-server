use serde::{Deserialize, Serialize};

use crate::api::{DisplayDiff, LayerDescriptor, LayerVisibilityOverride, PsdDocument};
use crate::layer_name_format::{
    is_exclusive_kind, is_exclusive_name, is_mandatory_kind, is_mandatory_name, is_toggleable_kind,
};
use crate::model::LayerKind;
#[path = "eye_blink/visibility.rs"]
mod visibility;

use visibility::{
    ensure_named_row_visible, exclusive_scope_bounds, find_active_eye_blink_pair,
    find_named_exclusive_pair, resolve_row_states, row_label, RowVisibilityState,
};

pub const EYE_SET_LAYER: &str = "目セット";
pub const SMILE_LAYER: &str = "にっこり";
pub const BASIC_EYE_LAYER: &str = "基本目";
pub const NORMAL_EYE_LAYER: &str = "普通目";
pub const CLOSED_EYE_LAYER: &str = "閉じ目";
pub const CLOSED_EYE_LAYER_ALT_1: &str = "目閉じ2";
pub const CLOSED_EYE_LAYER_ALT_2: &str = "目閉じ";
pub const AUTO_EYE_BLINK_PREFERRED_OPEN_LAYER_NAMES: &[&str] =
    &[EYE_SET_LAYER, BASIC_EYE_LAYER, NORMAL_EYE_LAYER];
pub const AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS: &[&str] = &[
    CLOSED_EYE_LAYER,
    CLOSED_EYE_LAYER_ALT_1,
    CLOSED_EYE_LAYER_ALT_2,
    SMILE_LAYER,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EyeBlinkTarget {
    pub psd_file_name: String,
    pub first_layer_name: String,
    pub second_layer_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EyeBlinkRows {
    pub open_row_index: usize,
    pub closed_row_index: usize,
    pub open_label: String,
    pub closed_label: String,
}

pub fn auto_generate_eye_blink_target(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
) -> Result<EyeBlinkTarget, String> {
    auto_generate_eye_blink_target_with_keyword_slices(
        document,
        base_variation,
        AUTO_EYE_BLINK_PREFERRED_OPEN_LAYER_NAMES,
        AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS,
    )
}

pub fn auto_generate_eye_blink_target_with_keywords<
    OpenLayerName: AsRef<str>,
    ClosedLayerKeyword: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    preferred_open_layer_names: &[OpenLayerName],
    closed_layer_keywords: &[ClosedLayerKeyword],
) -> Result<EyeBlinkTarget, String> {
    auto_generate_eye_blink_target_with_keyword_slices(
        document,
        base_variation,
        preferred_open_layer_names,
        closed_layer_keywords,
    )
}

fn auto_generate_eye_blink_target_with_keyword_slices<
    OpenLayerName: AsRef<str>,
    ClosedLayerKeyword: AsRef<str>,
>(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    preferred_open_layer_names: &[OpenLayerName],
    closed_layer_keywords: &[ClosedLayerKeyword],
) -> Result<EyeBlinkTarget, String> {
    let states = resolve_row_states(document, base_variation);
    let (open_row_index, closed_row_index) = find_auto_eye_blink_pair(
        document,
        &states,
        preferred_open_layer_names,
        closed_layer_keywords,
    )
    .ok_or_else(|| {
        format!(
            "PSD '{}' does not contain an auto-detectable eye blink pair matching keywords: {}",
            document.file_name,
            format_name_list(closed_layer_keywords)
        )
    })?;

    Ok(EyeBlinkTarget {
        psd_file_name: document.file_name.clone(),
        first_layer_name: row_label(document, open_row_index),
        second_layer_name: row_label(document, closed_row_index),
    })
}

pub fn build_closed_eye_display_diff(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    target: &EyeBlinkTarget,
) -> Result<DisplayDiff, String> {
    let resolved = resolve_eye_blink_rows(document, base_variation, target)?;
    ensure_named_row_visible(
        base_variation,
        document,
        resolved.closed_row_index,
        &resolved.closed_label,
    )
}

pub fn resolve_eye_blink_rows(
    document: &PsdDocument,
    base_variation: &DisplayDiff,
    target: &EyeBlinkTarget,
) -> Result<EyeBlinkRows, String> {
    let (open_row_index, closed_row_index) =
        find_active_eye_blink_pair(document, base_variation, target)
            .or_else(|| {
                find_named_exclusive_pair(
                    document,
                    &target.first_layer_name,
                    &target.second_layer_name,
                )
            })
            .ok_or_else(|| {
                format!(
                    "PSD does not contain sibling '*{}' and '*{}' layers",
                    target.first_layer_name, target.second_layer_name
                )
            })?;

    Ok(EyeBlinkRows {
        open_row_index,
        closed_row_index,
        open_label: row_label(document, open_row_index),
        closed_label: row_label(document, closed_row_index),
    })
}

fn find_auto_eye_blink_pair<OpenLayerName: AsRef<str>, ClosedLayerKeyword: AsRef<str>>(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    preferred_open_layer_names: &[OpenLayerName],
    closed_layer_keywords: &[ClosedLayerKeyword],
) -> Option<(usize, usize)> {
    let mut best_match = None;

    for (closed_row_index, closed_descriptor) in document.layers.iter().enumerate() {
        if !is_auto_eye_blink_second_layer(closed_descriptor, closed_layer_keywords)
            || !states
                .get(closed_row_index)
                .is_some_and(|state| state.parent_visible)
        {
            continue;
        }

        let Some((keyword_rank, open_row_index, open_score)) = find_auto_open_row(
            document,
            states,
            closed_row_index,
            preferred_open_layer_names,
            closed_layer_keywords,
        ) else {
            continue;
        };
        let candidate_score = (
            keyword_rank,
            open_score.visible_penalty,
            open_score.preferred_name_rank,
            open_score.distance,
            closed_row_index,
            open_row_index,
        );

        match best_match {
            Some((best_score, _, _)) if best_score <= candidate_score => {}
            _ => best_match = Some((candidate_score, open_row_index, closed_row_index)),
        }
    }

    best_match.map(|(_, open_row_index, closed_row_index)| (open_row_index, closed_row_index))
}

fn find_auto_open_row<OpenLayerName: AsRef<str>, ClosedLayerKeyword: AsRef<str>>(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    closed_row_index: usize,
    preferred_open_layer_names: &[OpenLayerName],
    closed_layer_keywords: &[ClosedLayerKeyword],
) -> Option<(usize, usize, AutoOpenScore)> {
    let closed_descriptor = document.layers.get(closed_row_index)?;
    let (scope_start, scope_end) = exclusive_scope_bounds(document, closed_row_index);
    let keyword_rank = auto_eye_blink_second_keyword_rank(
        normalize_eye_blink_layer_name(&closed_descriptor.name),
        closed_layer_keywords,
    )?;
    let mut best_match = None;

    for row_index in scope_start..scope_end {
        if row_index == closed_row_index {
            continue;
        }

        let descriptor = &document.layers[row_index];
        let normalized_name = normalize_eye_blink_layer_name(&descriptor.name);
        if descriptor.depth != closed_descriptor.depth
            || !is_exclusive_kind(descriptor.kind)
            || !is_exclusive_name(&descriptor.name)
            || auto_eye_blink_second_keyword_rank(normalized_name, closed_layer_keywords).is_some()
        {
            continue;
        }

        let score = AutoOpenScore {
            visible_penalty: usize::from(!states.get(row_index).is_some_and(|state| state.visible)),
            preferred_name_rank: preferred_open_layer_names
                .iter()
                .position(|name| name.as_ref() == normalized_name)
                .unwrap_or(preferred_open_layer_names.len()),
            distance: row_index.abs_diff(closed_row_index),
        };
        match best_match {
            Some((best_score, _)) if best_score <= score => {}
            _ => best_match = Some((score, row_index)),
        }
    }

    best_match.map(|(score, row_index)| (keyword_rank, row_index, score))
}

fn is_auto_eye_blink_second_layer(
    descriptor: &LayerDescriptor,
    closed_layer_keywords: &[impl AsRef<str>],
) -> bool {
    is_exclusive_kind(descriptor.kind)
        && is_exclusive_name(&descriptor.name)
        && auto_eye_blink_second_keyword_rank(
            normalize_eye_blink_layer_name(&descriptor.name),
            closed_layer_keywords,
        )
        .is_some()
}

fn auto_eye_blink_second_keyword_rank(
    normalized_name: &str,
    closed_layer_keywords: &[impl AsRef<str>],
) -> Option<usize> {
    closed_layer_keywords
        .iter()
        .position(|keyword| normalized_name.contains(keyword.as_ref()))
}

fn normalize_eye_blink_layer_name(name: &str) -> &str {
    name.trim().trim_start_matches(['*', '!']).trim()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AutoOpenScore {
    visible_penalty: usize,
    preferred_name_rank: usize,
    distance: usize,
}

fn format_name_list(values: &[impl AsRef<str>]) -> String {
    values
        .iter()
        .map(|value| value.as_ref())
        .collect::<Vec<_>>()
        .join(", ")
}
