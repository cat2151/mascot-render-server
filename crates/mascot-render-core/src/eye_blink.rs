use serde::{Deserialize, Serialize};

use crate::api::{DisplayDiff, LayerDescriptor, LayerVisibilityOverride, PsdDocument};
use crate::layer_name_format::{
    is_exclusive_kind, is_exclusive_name, is_mandatory_kind, is_mandatory_name, is_toggleable_kind,
};
use crate::model::LayerKind;
#[path = "eye_blink/visibility.rs"]
mod visibility;

use visibility::{
    ensure_named_row_visible, find_active_eye_blink_pair, find_named_exclusive_pair, row_label,
};

pub const PSD_ZUNDAMON_23: &str = "ずんだもん立ち絵素材2.3.psd";
pub const PSD_ZUNDAMON_111: &str = "ずんだもん立ち絵素材改ver1.1.1.psd";
pub const PSD_ZUNDAMON_V32_BASIC: &str = "ずんだもん立ち絵素材V3.2_基本版.psd";
pub const PSD_ZUNDAMON_V32_FULL: &str = "ずんだもん立ち絵素材V3.2_全部詰め版.psd";
pub const PSD_ZUNDAMON_V32_UPWARD: &str = "ずんだもん立ち絵素材V3.2_上向き版.psd";

pub const EYE_SET_LAYER: &str = "目セット";
pub const SMILE_LAYER: &str = "にっこり";
pub const BASIC_EYE_LAYER: &str = "基本目";
pub const NORMAL_EYE_LAYER: &str = "普通目";
pub const CLOSED_EYE_LAYER: &str = "閉じ目";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EyeBlinkDefaultTarget {
    pub psd_file_name: &'static str,
    pub first_layer_name: &'static str,
    pub second_layer_name: &'static str,
}

pub const DEFAULT_EYE_BLINK_TARGETS: [EyeBlinkDefaultTarget; 5] = [
    EyeBlinkDefaultTarget {
        psd_file_name: PSD_ZUNDAMON_23,
        first_layer_name: EYE_SET_LAYER,
        second_layer_name: SMILE_LAYER,
    },
    EyeBlinkDefaultTarget {
        psd_file_name: PSD_ZUNDAMON_111,
        first_layer_name: BASIC_EYE_LAYER,
        second_layer_name: CLOSED_EYE_LAYER,
    },
    EyeBlinkDefaultTarget {
        psd_file_name: PSD_ZUNDAMON_V32_BASIC,
        first_layer_name: BASIC_EYE_LAYER,
        second_layer_name: CLOSED_EYE_LAYER,
    },
    EyeBlinkDefaultTarget {
        psd_file_name: PSD_ZUNDAMON_V32_FULL,
        first_layer_name: BASIC_EYE_LAYER,
        second_layer_name: CLOSED_EYE_LAYER,
    },
    EyeBlinkDefaultTarget {
        psd_file_name: PSD_ZUNDAMON_V32_UPWARD,
        first_layer_name: NORMAL_EYE_LAYER,
        second_layer_name: CLOSED_EYE_LAYER,
    },
];

pub fn default_eye_blink_targets() -> Vec<EyeBlinkTarget> {
    DEFAULT_EYE_BLINK_TARGETS
        .iter()
        .map(|target| EyeBlinkTarget {
            psd_file_name: target.psd_file_name.to_string(),
            first_layer_name: target.first_layer_name.to_string(),
            second_layer_name: target.second_layer_name.to_string(),
        })
        .collect()
}

pub fn migrate_eye_blink_layers(
    psd_file_name: &str,
    first_layer_name: &str,
    second_layer_name: &str,
) -> Option<(&'static str, &'static str)> {
    if second_layer_name != CLOSED_EYE_LAYER {
        return None;
    }

    match (psd_file_name, first_layer_name) {
        (PSD_ZUNDAMON_111, NORMAL_EYE_LAYER)
        | (PSD_ZUNDAMON_V32_BASIC, NORMAL_EYE_LAYER)
        | (PSD_ZUNDAMON_V32_FULL, NORMAL_EYE_LAYER) => Some((BASIC_EYE_LAYER, CLOSED_EYE_LAYER)),
        _ => None,
    }
}

pub fn find_eye_blink_target<'a>(
    targets: &'a [EyeBlinkTarget],
    psd_file_name: &str,
) -> Option<&'a EyeBlinkTarget> {
    targets
        .iter()
        .find(|target| target.psd_file_name == psd_file_name)
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
