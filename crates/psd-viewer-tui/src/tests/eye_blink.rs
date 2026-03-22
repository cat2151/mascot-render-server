use mascot_render_core::{
    default_eye_blink_targets, find_eye_blink_target, BASIC_EYE_LAYER, CLOSED_EYE_LAYER,
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
