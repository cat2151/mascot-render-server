use std::path::PathBuf;

use mascot_render_core::{DisplayDiff, LayerKind, PsdDocument};

use crate::display_diff_state::{
    descriptor, find_named_exclusive_pair, resolve_layer_rows, toggle_layer_override,
};

fn sample_document() -> PsdDocument {
    PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/basic.psd"),
        file_name: "basic.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(2, "Body", LayerKind::GroupOpen, true, true, 0),
            descriptor(1, "Face", LayerKind::Layer, true, true, 1),
            descriptor(0, "Body End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    }
}

#[test]
fn toggle_layer_override_hides_group_children() {
    let document = sample_document();
    let mut display_diff = DisplayDiff::new();

    assert!(toggle_layer_override(&mut display_diff, &document, 0));

    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(rows[0].visible, false);
    assert_eq!(rows[1].visible, false);
    assert_eq!(rows[2].visible, false);
}

#[test]
fn starred_layers_in_same_group_become_exclusive_when_enabled() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/exclusive.psd"),
        file_name: "exclusive.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(5, "Eyes", LayerKind::GroupOpen, true, true, 0),
            descriptor(4, "*Open", LayerKind::Layer, true, true, 1),
            descriptor(3, "*Closed", LayerKind::Layer, false, false, 1),
            descriptor(2, "Highlight", LayerKind::Layer, true, true, 1),
            descriptor(1, "Eyes End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(toggle_layer_override(&mut display_diff, &document, 2));

    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[1].visible, false,
        "other starred sibling should be hidden"
    );
    assert_eq!(
        rows[2].visible, true,
        "selected starred layer should be visible"
    );
    assert_eq!(
        rows[3].visible, true,
        "non-starred sibling should stay unchanged"
    );
}

#[test]
fn visible_starred_layer_cannot_be_hidden_directly() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/radio.psd"),
        file_name: "radio.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(5, "Eyes", LayerKind::GroupOpen, true, true, 0),
            descriptor(4, "*Open", LayerKind::Layer, true, true, 1),
            descriptor(3, "*Closed", LayerKind::Layer, false, false, 1),
            descriptor(2, "Eyes End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(!toggle_layer_override(&mut display_diff, &document, 1));
    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[1].visible, true,
        "currently visible starred layer should stay visible"
    );
    assert_eq!(
        rows[2].visible, false,
        "other starred layer should stay hidden"
    );
}

#[test]
fn mandatory_layers_stay_visible_and_cannot_be_hidden() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/mandatory.psd"),
        file_name: "mandatory.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(3, "Body", LayerKind::GroupOpen, true, true, 0),
            descriptor(2, "!Core", LayerKind::Layer, true, true, 1),
            descriptor(1, "Body End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(!toggle_layer_override(&mut display_diff, &document, 1));
    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[1].visible, true,
        "mandatory layer should remain visible"
    );
    assert!(
        display_diff.visibility_overrides.is_empty(),
        "mandatory no-op should not create overrides"
    );
}

#[test]
fn starred_layers_do_not_hide_nested_group_variants() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/nested-exclusive.psd"),
        file_name: "nested-exclusive.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(8, "Body", LayerKind::GroupOpen, true, true, 0),
            descriptor(7, "*Base", LayerKind::Layer, true, true, 1),
            descriptor(6, "Face", LayerKind::GroupOpen, true, true, 1),
            descriptor(5, "*Smile", LayerKind::Layer, true, true, 2),
            descriptor(4, "*Cry", LayerKind::Layer, false, false, 2),
            descriptor(3, "Face End", LayerKind::GroupClose, true, true, 1),
            descriptor(2, "Body End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(toggle_layer_override(&mut display_diff, &document, 4));

    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[1].visible, true,
        "outer group starred layer should not be affected"
    );
    assert_eq!(
        rows[3].visible, false,
        "nested starred sibling should be hidden"
    );
    assert_eq!(
        rows[4].visible, true,
        "selected nested starred layer should be visible"
    );
}

#[test]
fn starred_groups_become_exclusive_at_same_depth() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/group-radio.psd"),
        file_name: "group-radio.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(9, "*Face A", LayerKind::GroupOpen, true, true, 0),
            descriptor(8, "A Item", LayerKind::Layer, true, true, 1),
            descriptor(7, "Face A End", LayerKind::GroupClose, true, true, 0),
            descriptor(6, "*Face B", LayerKind::GroupOpen, false, false, 0),
            descriptor(5, "B Item", LayerKind::Layer, true, false, 1),
            descriptor(4, "Face B End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(toggle_layer_override(&mut display_diff, &document, 3));

    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[0].visible, false,
        "other starred group should be hidden"
    );
    assert_eq!(
        rows[3].visible, true,
        "selected starred group should be visible"
    );
}

#[test]
fn starred_group_and_starred_layer_are_exclusive_at_same_depth() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/mixed-radio.psd"),
        file_name: "mixed-radio.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(8, "Root", LayerKind::GroupOpen, true, true, 0),
            descriptor(7, "*EyeSet", LayerKind::GroupOpen, false, false, 1),
            descriptor(6, "EyeSet Item", LayerKind::Layer, true, false, 2),
            descriptor(5, "EyeSet End", LayerKind::GroupClose, true, true, 1),
            descriptor(4, "*Upward", LayerKind::Layer, true, true, 1),
            descriptor(3, "Root End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };
    let mut display_diff = DisplayDiff::new();

    assert!(toggle_layer_override(&mut display_diff, &document, 1));

    let rows = resolve_layer_rows(&document, &display_diff);
    assert_eq!(
        rows[1].visible, true,
        "selected starred group should be visible"
    );
    assert_eq!(
        rows[4].visible, false,
        "same-depth starred layer should be hidden by starred group selection"
    );
}

#[test]
fn finds_named_exclusive_pair_in_same_scope() {
    let document = PsdDocument {
        zip_path: PathBuf::from("assets/zip/demo.zip"),
        psd_path_in_zip: PathBuf::from("demo/mouth.psd"),
        file_name: "mouth.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        layers: vec![
            descriptor(9, "Face", LayerKind::GroupOpen, true, true, 0),
            descriptor(8, "*むふ", LayerKind::Layer, true, true, 1),
            descriptor(7, "*ほあー", LayerKind::Layer, false, false, 1),
            descriptor(6, "Face End", LayerKind::GroupClose, true, true, 0),
            descriptor(5, "Other", LayerKind::GroupOpen, true, true, 0),
            descriptor(4, "*むふ", LayerKind::Layer, true, true, 1),
            descriptor(3, "*ほあー", LayerKind::Layer, false, false, 2),
            descriptor(2, "Other End", LayerKind::GroupClose, true, true, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let pair = find_named_exclusive_pair(&document, "ほあー", "むふ");
    assert_eq!(pair, Some((2, 1)));
}

