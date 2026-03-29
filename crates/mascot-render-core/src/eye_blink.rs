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
pub const AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS: [&str; 2] = [CLOSED_EYE_LAYER, SMILE_LAYER];
const AUTO_EYE_BLINK_PREFERRED_OPEN_LAYER_NAMES: [&str; 3] =
    [EYE_SET_LAYER, BASIC_EYE_LAYER, NORMAL_EYE_LAYER];

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
    let states = resolve_row_states(document, base_variation);
    let (open_row_index, closed_row_index) = find_auto_eye_blink_pair(document, &states)
        .ok_or_else(|| {
            format!(
                "PSD '{}' does not contain an auto-detectable eye blink pair matching keywords: {}",
                document.file_name,
                AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS.join(", ")
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

fn find_auto_eye_blink_pair(
    document: &PsdDocument,
    states: &[RowVisibilityState],
) -> Option<(usize, usize)> {
    let mut best_match = None;

    for (closed_row_index, closed_descriptor) in document.layers.iter().enumerate() {
        if !is_auto_eye_blink_second_layer(closed_descriptor)
            || !states
                .get(closed_row_index)
                .is_some_and(|state| state.parent_visible)
        {
            continue;
        }

        let Some((keyword_rank, open_row_index, open_score)) =
            find_auto_open_row(document, states, closed_row_index)
        else {
            continue;
        };
        let candidate_score = (
            keyword_rank,
            open_score.visible_penalty,
            open_score.preferred_name_penalty,
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

fn find_auto_open_row(
    document: &PsdDocument,
    states: &[RowVisibilityState],
    closed_row_index: usize,
) -> Option<(usize, usize, AutoOpenScore)> {
    let closed_descriptor = document.layers.get(closed_row_index)?;
    let (scope_start, scope_end) = exclusive_scope_bounds(document, closed_row_index);
    let keyword_rank = auto_eye_blink_second_keyword_rank(normalize_eye_blink_layer_name(
        &closed_descriptor.name,
    ))?;
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
            || auto_eye_blink_second_keyword_rank(normalized_name).is_some()
        {
            continue;
        }

        let score = AutoOpenScore {
            visible_penalty: usize::from(!states.get(row_index).is_some_and(|state| state.visible)),
            preferred_name_penalty: usize::from(
                !AUTO_EYE_BLINK_PREFERRED_OPEN_LAYER_NAMES.contains(&normalized_name),
            ),
            distance: row_index.abs_diff(closed_row_index),
        };
        match best_match {
            Some((best_score, _)) if best_score <= score => {}
            _ => best_match = Some((score, row_index)),
        }
    }

    best_match.map(|(score, row_index)| (keyword_rank, row_index, score))
}

fn is_auto_eye_blink_second_layer(descriptor: &LayerDescriptor) -> bool {
    is_exclusive_kind(descriptor.kind)
        && is_exclusive_name(&descriptor.name)
        && auto_eye_blink_second_keyword_rank(normalize_eye_blink_layer_name(&descriptor.name))
            .is_some()
}

fn auto_eye_blink_second_keyword_rank(normalized_name: &str) -> Option<usize> {
    AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS
        .iter()
        .position(|keyword| normalized_name.contains(keyword))
}

fn normalize_eye_blink_layer_name(name: &str) -> &str {
    name.trim().trim_start_matches(['*', '!']).trim()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AutoOpenScore {
    visible_penalty: usize,
    preferred_name_penalty: usize,
    distance: usize,
}
