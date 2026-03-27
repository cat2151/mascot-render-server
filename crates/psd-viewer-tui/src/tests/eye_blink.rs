use crate::app::eye_blink::auto_generate_eye_blink_target;
use crate::display_diff_state::LayerRow;
use mascot_render_core::{
    default_eye_blink_targets, find_eye_blink_target, LayerKind, BASIC_EYE_LAYER, CLOSED_EYE_LAYER,
    NORMAL_EYE_LAYER,
};

#[test]
fn default_eye_blink_targets_match_by_psd_file_name() {
    let targets = default_eye_blink_targets();

    let target = find_eye_blink_target(&targets, "ずんだもん立ち絵素材V3.2_基本版.psd")
        .expect("default V3.2 target should exist");
    assert_eq!(target.first_layer_name, BASIC_EYE_LAYER);
    assert_eq!(target.second_layer_name, CLOSED_EYE_LAYER);
}

#[test]
fn default_eye_blink_targets_keep_upward_psd_on_normal_eye() {
    let targets = default_eye_blink_targets();

    let target = find_eye_blink_target(&targets, "ずんだもん立ち絵素材V3.2_上向き版.psd")
        .expect("upward V3.2 target should exist");
    assert_eq!(target.first_layer_name, NORMAL_EYE_LAYER);
    assert_eq!(target.second_layer_name, CLOSED_EYE_LAYER);
}

#[test]
fn eye_blink_target_lookup_returns_none_for_unknown_psd() {
    let targets = default_eye_blink_targets();

    assert!(find_eye_blink_target(&targets, "missing.psd").is_none());
}

#[test]
fn auto_generate_eye_blink_target_uses_selected_layer_and_closed_eye_match() {
    let layer_rows = vec![
        layer_row("*基本目", true, 0),
        layer_row("*閉じ目", false, 0),
        layer_row("*にっこり", false, 0),
    ];

    let (target, log) =
        auto_generate_eye_blink_target("demo.psd", &layer_rows, 0).expect("should auto-generate");

    assert_eq!(target.first_layer_name, "基本目");
    assert_eq!(target.second_layer_name, "閉じ目");
    assert!(log.contains("Auto-generated eye blink target for PSD 'demo.psd'."));
    assert!(log.contains("[0] [Layer] *基本目 (visible) <selected>"));
    assert!(log.contains("[1] [Layer] *閉じ目 (hidden)"));
    assert!(log.contains("matched keyword '閉じ目'"));
}

#[test]
fn auto_generate_eye_blink_target_falls_back_to_smile_match() {
    let layer_rows = vec![
        layer_row("*基本目", true, 0),
        layer_row("*にっこり", false, 0),
    ];

    let (target, log) =
        auto_generate_eye_blink_target("demo.psd", &layer_rows, 0).expect("should auto-generate");

    assert_eq!(target.first_layer_name, "基本目");
    assert_eq!(target.second_layer_name, "にっこり");
    assert!(log.contains("matched keyword 'にっこり'"));
}

#[test]
fn auto_generate_eye_blink_target_errors_when_no_candidate_exists() {
    let layer_rows = vec![layer_row("*基本目", true, 0), layer_row("*口", false, 0)];

    let error =
        auto_generate_eye_blink_target("demo.psd", &layer_rows, 0).expect_err("should fail");

    assert!(error.contains("no layer matched auto eye blink keywords"));
}

#[test]
fn auto_generate_eye_blink_target_prefers_nearest_same_depth_candidate() {
    let layer_rows = vec![
        layer_row("*遠い閉じ目", false, 1),
        layer_row("*普通目", true, 1),
        layer_row("*基本目", true, 2),
        layer_row("*近い閉じ目", false, 2),
        layer_row("*さらに遠い閉じ目", false, 2),
    ];

    let (target, log) =
        auto_generate_eye_blink_target("demo.psd", &layer_rows, 2).expect("should auto-generate");

    assert_eq!(target.first_layer_name, "基本目");
    assert_eq!(target.second_layer_name, "近い閉じ目");
    assert!(log.contains("second_layer_name: '近い閉じ目'"));
}

fn layer_row(name: &str, visible: bool, depth: usize) -> LayerRow {
    LayerRow {
        name: name.to_string(),
        kind: LayerKind::Layer,
        visible,
        depth,
    }
}
