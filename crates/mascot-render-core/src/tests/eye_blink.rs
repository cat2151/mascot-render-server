use std::path::PathBuf;

use crate::{
    build_closed_eye_display_diff, default_eye_blink_targets, migrate_eye_blink_layers,
    resolve_eye_blink_rows, DisplayDiff, LayerDescriptor, LayerKind, PsdDocument,
};

#[test]
fn default_eye_blink_targets_use_basic_eye_for_known_psds() {
    let targets = default_eye_blink_targets();

    let target = targets
        .iter()
        .find(|target| target.psd_file_name == "ずんだもん立ち絵素材V3.2_基本版.psd")
        .expect("default V3.2 target should exist");
    assert_eq!(target.first_layer_name, "基本目");
    assert_eq!(target.second_layer_name, "閉じ目");
}

#[test]
fn migrate_eye_blink_layers_rewrites_legacy_normal_eye_targets() {
    let migrated =
        migrate_eye_blink_layers("ずんだもん立ち絵素材改ver1.1.1.psd", "普通目", "閉じ目")
            .expect("legacy target should migrate");

    assert_eq!(migrated, ("基本目", "閉じ目"));
}

#[test]
fn build_closed_eye_display_diff_activates_closed_eye_layer() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(0, "*基本目", LayerKind::Layer, true, 0),
            descriptor(1, "*閉じ目", LayerKind::Layer, false, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let target = crate::EyeBlinkTarget {
        psd_file_name: "demo.psd".to_string(),
        first_layer_name: "基本目".to_string(),
        second_layer_name: "閉じ目".to_string(),
    };

    let display_diff = build_closed_eye_display_diff(&document, &DisplayDiff::new(), &target)
        .expect("closed-eye diff should build");

    assert_eq!(display_diff.visibility_overrides.len(), 2);
    assert_eq!(display_diff.visibility_overrides[0].layer_index, 0);
    assert!(!display_diff.visibility_overrides[0].visible);
    assert_eq!(display_diff.visibility_overrides[1].layer_index, 1);
    assert!(display_diff.visibility_overrides[1].visible);
}

#[test]
fn resolve_eye_blink_rows_prefers_visible_parent_scope() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "ずんだもん立ち絵素材V3.2_全部詰め版.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(13, "*頭_正面向き", LayerKind::GroupOpen, true, 0),
            descriptor(12, "!目", LayerKind::GroupOpen, true, 1),
            descriptor(11, "*基本目", LayerKind::Layer, true, 2),
            descriptor(10, "*閉じ目", LayerKind::Layer, false, 2),
            descriptor(9, "(unnamed)", LayerKind::GroupClose, true, 1),
            descriptor(8, "(unnamed)", LayerKind::GroupClose, true, 0),
            descriptor(7, "*頭_上向き", LayerKind::GroupOpen, false, 0),
            descriptor(6, "!目", LayerKind::GroupOpen, true, 1),
            descriptor(5, "*普通目", LayerKind::Layer, true, 2),
            descriptor(4, "*閉じ目", LayerKind::Layer, false, 2),
            descriptor(3, "(unnamed)", LayerKind::GroupClose, true, 1),
            descriptor(2, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let target = crate::EyeBlinkTarget {
        psd_file_name: "ずんだもん立ち絵素材V3.2_全部詰め版.psd".to_string(),
        first_layer_name: "基本目".to_string(),
        second_layer_name: "閉じ目".to_string(),
    };
    let base_variation = DisplayDiff {
        version: 1,
        visibility_overrides: vec![
            crate::LayerVisibilityOverride {
                layer_index: 13,
                visible: false,
            },
            crate::LayerVisibilityOverride {
                layer_index: 7,
                visible: true,
            },
        ],
    };

    let resolved = resolve_eye_blink_rows(&document, &base_variation, &target)
        .expect("visible upward eye scope should be resolved");

    assert_eq!(resolved.open_row_index, 8);
    assert_eq!(resolved.closed_row_index, 9);
    assert_eq!(resolved.open_label, "普通目");
    assert_eq!(resolved.closed_label, "閉じ目");

    let display_diff = build_closed_eye_display_diff(&document, &base_variation, &target)
        .expect("closed eye should activate in visible upward scope");

    assert_eq!(display_diff.visibility_overrides.len(), 4);
    assert_eq!(display_diff.visibility_overrides[0].layer_index, 4);
    assert!(display_diff.visibility_overrides[0].visible);
    assert_eq!(display_diff.visibility_overrides[1].layer_index, 5);
    assert!(!display_diff.visibility_overrides[1].visible);
    assert_eq!(display_diff.visibility_overrides[2].layer_index, 7);
    assert!(display_diff.visibility_overrides[2].visible);
    assert_eq!(display_diff.visibility_overrides[3].layer_index, 13);
    assert!(!display_diff.visibility_overrides[3].visible);
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
