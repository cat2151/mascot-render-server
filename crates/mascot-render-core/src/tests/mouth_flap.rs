use std::path::PathBuf;

use crate::{
    default_mouth_flap_targets, resolve_mouth_flap_rows, DisplayDiff, LayerDescriptor, LayerKind,
    LayerVisibilityOverride, MouthFlapTarget, PsdDocument,
};

#[test]
fn default_mouth_flap_targets_use_expected_zundamon_layers() {
    let targets = default_mouth_flap_targets();

    let target = targets
        .iter()
        .find(|target| target.psd_file_name == "ずんだもん立ち絵素材V3.2_基本版.psd")
        .expect("default V3.2 target should exist");
    assert_eq!(target.open_layer_names, vec!["ほあー"]);
    assert_eq!(target.closed_layer_names, vec!["むふ", "むん", "ん"]);
}

#[test]
fn resolve_mouth_flap_rows_prefers_visible_mouth_group() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "ずんだもん立ち絵素材V3.2_全部詰め版.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(15, "*頭_正面向き", LayerKind::GroupOpen, false, 0),
            descriptor(14, "!口", LayerKind::GroupOpen, true, 1),
            descriptor(13, "*ほあー", LayerKind::Layer, true, 2),
            descriptor(12, "*むふ", LayerKind::Layer, false, 2),
            descriptor(11, "(unnamed)", LayerKind::GroupClose, true, 1),
            descriptor(10, "(unnamed)", LayerKind::GroupClose, true, 0),
            descriptor(9, "*頭_上向き", LayerKind::GroupOpen, true, 0),
            descriptor(8, "!口", LayerKind::GroupOpen, true, 1),
            descriptor(7, "*ほあー", LayerKind::Layer, true, 2),
            descriptor(6, "*むん", LayerKind::Layer, false, 2),
            descriptor(5, "(unnamed)", LayerKind::GroupClose, true, 1),
            descriptor(4, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let target = MouthFlapTarget {
        psd_file_name: "ずんだもん立ち絵素材V3.2_全部詰め版.psd".to_string(),
        open_layer_names: vec!["ほあー".to_string()],
        closed_layer_names: vec!["むふ".to_string(), "むん".to_string(), "ん".to_string()],
    };
    let base_variation = DisplayDiff {
        version: 1,
        visibility_overrides: vec![
            LayerVisibilityOverride {
                layer_index: 15,
                visible: false,
            },
            LayerVisibilityOverride {
                layer_index: 9,
                visible: true,
            },
        ],
    };

    let resolved = resolve_mouth_flap_rows(&document, &base_variation, &target)
        .expect("visible mouth group should be resolved");

    assert_eq!(resolved.open_row_index, 8);
    assert_eq!(resolved.closed_row_index, 9);
    assert_eq!(resolved.open_label, "ほあー");
    assert_eq!(resolved.closed_label, "むん");
}

fn descriptor(
    layer_index: usize,
    name: &str,
    kind: LayerKind,
    default_visible: bool,
    depth: usize,
) -> LayerDescriptor {
    LayerDescriptor {
        layer_index,
        name: name.to_string(),
        kind,
        default_visible,
        effective_visible: default_visible,
        depth,
    }
}
