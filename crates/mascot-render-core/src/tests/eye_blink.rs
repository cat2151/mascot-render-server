use std::path::PathBuf;

use crate::{
    auto_generate_eye_blink_target, build_closed_eye_display_diff, resolve_eye_blink_rows,
    DisplayDiff, LayerDescriptor, LayerKind, PsdDocument,
};

#[test]
fn auto_generate_eye_blink_target_prefers_visible_closed_eye_pair() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
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

    let target =
        auto_generate_eye_blink_target(&document, &base_variation).expect("should auto-generate");

    assert_eq!(target.psd_file_name, "demo.psd");
    assert_eq!(target.first_layer_name, "普通目");
    assert_eq!(target.second_layer_name, "閉じ目");
}

#[test]
fn auto_generate_eye_blink_target_falls_back_to_smile_layer() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(2, "*目セット", LayerKind::Layer, true, 0),
            descriptor(1, "*にっこり", LayerKind::Layer, false, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let target = auto_generate_eye_blink_target(&document, &DisplayDiff::default())
        .expect("should match smile");

    assert_eq!(target.first_layer_name, "目セット");
    assert_eq!(target.second_layer_name, "にっこり");
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
