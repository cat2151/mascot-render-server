use std::path::PathBuf;

use crate::{
    auto_generate_mouth_flap_target, auto_generate_mouth_flap_target_with_layer_names,
    build_mouth_flap_display_diffs, describe_mouth_flap_auto_generation_failure,
    resolve_mouth_flap_rows, DisplayDiff, LayerDescriptor, LayerKind, LayerVisibilityOverride,
    MouthFlapTarget, PsdDocument,
};

#[test]
fn auto_generate_mouth_flap_target_prefers_visible_mouth_group() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
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

    let target =
        auto_generate_mouth_flap_target(&document, &base_variation).expect("should auto-generate");

    assert_eq!(target.psd_file_name, "demo.psd");
    assert_eq!(target.open_layer_names, vec!["ほあー"]);
    assert_eq!(target.closed_layer_names, vec!["むん"]);
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

#[test]
fn build_mouth_flap_display_diffs_activates_open_and_closed_layers() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(4, "!口", LayerKind::GroupOpen, true, 0),
            descriptor(3, "*ほあー", LayerKind::Layer, false, 1),
            descriptor(2, "*むふ", LayerKind::Layer, true, 1),
            descriptor(1, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let target = MouthFlapTarget {
        psd_file_name: "demo.psd".to_string(),
        open_layer_names: vec!["ほあー".to_string()],
        closed_layer_names: vec!["むふ".to_string()],
    };

    let display_diffs = build_mouth_flap_display_diffs(&document, &DisplayDiff::new(), &target)
        .expect("mouth flap display diffs should build");

    assert_eq!(
        display_diffs.open.visibility_overrides,
        vec![
            LayerVisibilityOverride {
                layer_index: 2,
                visible: false,
            },
            LayerVisibilityOverride {
                layer_index: 3,
                visible: true,
            },
        ]
    );
    assert_eq!(display_diffs.closed, DisplayDiff::new());
}

#[test]
fn auto_generate_mouth_flap_target_uses_added_open_and_closed_fallbacks() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(4, "!口", LayerKind::GroupOpen, true, 0),
            descriptor(3, "*ほー", LayerKind::Layer, true, 1),
            descriptor(2, "*んむ", LayerKind::Layer, false, 1),
            descriptor(1, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let target =
        auto_generate_mouth_flap_target(&document, &DisplayDiff::new()).expect("should resolve");

    assert_eq!(target.open_layer_names, vec!["ほー"]);
    assert_eq!(target.closed_layer_names, vec!["んむ"]);
}

#[test]
fn auto_generate_mouth_flap_target_supports_closed_fallback_with_long_n() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(4, "!口", LayerKind::GroupOpen, true, 0),
            descriptor(3, "*お", LayerKind::Layer, true, 1),
            descriptor(2, "*んー", LayerKind::Layer, false, 1),
            descriptor(1, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let target =
        auto_generate_mouth_flap_target(&document, &DisplayDiff::new()).expect("should resolve");

    assert_eq!(target.open_layer_names, vec!["お"]);
    assert_eq!(target.closed_layer_names, vec!["んー"]);
}

#[test]
fn auto_generate_mouth_flap_target_uses_configured_layer_names() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(4, "!口", LayerKind::GroupOpen, true, 0),
            descriptor(3, "*あ", LayerKind::Layer, true, 1),
            descriptor(2, "*ん", LayerKind::Layer, false, 1),
            descriptor(1, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let target = auto_generate_mouth_flap_target_with_layer_names(
        &document,
        &DisplayDiff::new(),
        &["あ"],
        &["ん"],
    )
    .expect("should resolve configured layer names");

    assert_eq!(target.open_layer_names, vec!["あ"]);
    assert_eq!(target.closed_layer_names, vec!["ん"]);
}

#[test]
fn describe_mouth_flap_auto_generation_failure_limits_logs_to_mouth_groups_and_missing_side() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(12, "!眉", LayerKind::GroupOpen, true, 0),
            descriptor(11, "*うえ", LayerKind::Layer, true, 1),
            descriptor(10, "(unnamed)", LayerKind::GroupClose, true, 0),
            descriptor(9, "!口A", LayerKind::GroupOpen, true, 0),
            descriptor(8, "*あ", LayerKind::Layer, true, 1),
            descriptor(7, "*むふ", LayerKind::Layer, false, 1),
            descriptor(6, "(unnamed)", LayerKind::GroupClose, true, 0),
            descriptor(5, "!口B", LayerKind::GroupOpen, false, 0),
            descriptor(4, "*ほあー", LayerKind::Layer, true, 1),
            descriptor(3, "*い", LayerKind::Layer, false, 1),
            descriptor(2, "(unnamed)", LayerKind::GroupClose, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let diagnostic = describe_mouth_flap_auto_generation_failure(&document, &DisplayDiff::new());

    assert!(
        !diagnostic.contains("!眉"),
        "non-mouth groups should not appear in the diagnostic"
    );
    assert!(
        diagnostic
            .contains("open candidates [ほあー, ほー, おー, お] were not found. layers: あ, むふ"),
        "missing open side should list only mouth-group layer names"
    );
    assert!(
        !diagnostic.contains(
            "closed candidates [むふ, むん, んむ, ん, んー] were not found. layers: あ, むふ"
        ),
        "closed layers should not be logged when they are already present"
    );
    assert!(
        diagnostic.contains(
            "closed candidates [むふ, むん, んむ, ん, んー] were not found. layers: ほあー, い"
        ),
        "missing closed side should list only mouth-group layer names"
    );
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
