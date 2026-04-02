use crate::favorite_ensemble::{
    fill_missing_positions, pack_positions_from_right, patch_favorite_ensemble_positions_toml,
    sanitize_favorites_for_test, scaled_content_x_bounds, FavoriteEnsembleEntry,
    FavoriteEnsembleLayoutEntry,
};
use crate::mascot_app::{
    member_eye_blink_elapsed, member_eye_blink_seed, member_phase_offset_ratio,
};
use mascot_render_core::{LayerVisibilityOverride, MascotImageData};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn favorite_ensemble_packs_entries_from_right_edge_without_visible_horizontal_gaps() {
    let positions = pack_positions_from_right(&[
        FavoriteEnsembleLayoutEntry {
            size: [80.0, 120.0],
            content_x_bounds: [10.0, 70.0],
            position: None,
        },
        FavoriteEnsembleLayoutEntry {
            size: [40.0, 60.0],
            content_x_bounds: [5.0, 35.0],
            position: None,
        },
        FavoriteEnsembleLayoutEntry {
            size: [30.0, 90.0],
            content_x_bounds: [0.0, 20.0],
            position: None,
        },
    ]);

    assert_eq!(positions.len(), 3);
    assert_eq!(
        positions[0],
        [40.0, 0.0],
        "first favorite should align its visible right edge to the layout right edge"
    );
    assert_eq!(
        positions[1],
        [15.0, 60.0],
        "second favorite should continue leftward without a visible gap"
    );
    assert_eq!(
        positions[2],
        [0.0, 30.0],
        "later favorites should keep filling leftward based on visible bounds"
    );
}

#[test]
fn favorite_ensemble_only_places_missing_entries_to_the_left_of_existing_layout() {
    let mut layout = vec![
        FavoriteEnsembleLayoutEntry {
            size: [80.0, 120.0],
            content_x_bounds: [10.0, 70.0],
            position: Some([70.0, 0.0]),
        },
        FavoriteEnsembleLayoutEntry {
            size: [40.0, 60.0],
            content_x_bounds: [5.0, 35.0],
            position: None,
        },
        FavoriteEnsembleLayoutEntry {
            size: [30.0, 90.0],
            content_x_bounds: [10.0, 20.0],
            position: Some([40.0, 30.0]),
        },
    ];

    let updated = fill_missing_positions(&mut layout);

    assert_eq!(updated, vec![1]);
    assert_eq!(layout[0].position, Some([70.0, 0.0]));
    assert_eq!(layout[1].position, Some([15.0, 60.0]));
    assert_eq!(layout[2].position, Some([40.0, 30.0]));
}

#[test]
fn favorite_ensemble_patch_preserves_other_fields_and_existing_positions() {
    let raw = r#"
[[favorites]]
zip_path = "/workspace/a.zip"
psd_path_in_zip = "a/body.psd"
psd_file_name = "body.psd"
visibility_overrides = [{ layer_index = 3, visible = false }]
window_position = [120.0, 48.0]
favorite_ensemble_position = [360.0, 12.0]

[[favorites]]
zip_path = "/workspace/b.zip"
psd_path_in_zip = "b/face.psd"
psd_file_name = "face.psd"
window_position = [10.0, 20.0]
    "#;
    let updates = vec![
        FavoriteEnsembleEntry {
            zip_path: PathBuf::from("/workspace/a.zip"),
            psd_path_in_zip: PathBuf::from("a/body.psd"),
            psd_file_name: "body.psd".to_string(),
            visibility_overrides: vec![LayerVisibilityOverride {
                layer_index: 3,
                visible: false,
            }],
            mascot_scale: None,
            favorite_ensemble_position: Some([999.0, 999.0]),
        },
        FavoriteEnsembleEntry {
            zip_path: PathBuf::from("/workspace/b.zip"),
            psd_path_in_zip: PathBuf::from("b/face.psd"),
            psd_file_name: "face.psd".to_string(),
            visibility_overrides: Vec::new(),
            mascot_scale: None,
            favorite_ensemble_position: Some([40.0, 50.0]),
        },
    ];

    let patched =
        patch_favorite_ensemble_positions_toml(raw, &updates).expect("should patch favorites TOML");
    let parsed: toml::Value = toml::from_str(&patched).expect("patched TOML should stay valid");
    let favorites = parsed["favorites"]
        .as_array()
        .expect("favorites should remain an array");

    assert_eq!(
        favorites[0]["favorite_ensemble_position"].as_array(),
        Some(&vec![toml::Value::from(360.0), toml::Value::from(12.0)])
    );
    assert_eq!(
        favorites[0]["window_position"].as_array(),
        Some(&vec![toml::Value::from(120.0), toml::Value::from(48.0)])
    );
    assert_eq!(
        favorites[1]["favorite_ensemble_position"].as_array(),
        Some(&vec![toml::Value::from(40.0), toml::Value::from(50.0)])
    );
    assert_eq!(
        favorites[1]["window_position"].as_array(),
        Some(&vec![toml::Value::from(10.0), toml::Value::from(20.0)])
    );
}

#[test]
fn favorite_ensemble_scales_visible_content_x_bounds_from_alpha_pixels() {
    let bounds = scaled_content_x_bounds(
        &sample_favorite_entry(Some(10.0)),
        &MascotImageData {
            path: PathBuf::from("dummy-favorite.png"),
            width: 4,
            height: 2,
            rgba: rgba_with_alpha(&[
                0, 255, 255, 0, //
                0, 255, 255, 0,
            ]),
        },
        [40.0, 20.0],
    );

    assert_eq!(bounds, [10.0, 30.0]);
}

#[test]
fn favorite_ensemble_uses_full_width_when_image_is_fully_transparent() {
    let bounds = scaled_content_x_bounds(
        &sample_favorite_entry(Some(15.0)),
        &MascotImageData {
            path: PathBuf::from("dummy-favorite.png"),
            width: 3,
            height: 1,
            rgba: rgba_with_alpha(&[0, 0, 0]),
        },
        [45.0, 15.0],
    );

    assert_eq!(bounds, [0.0, 45.0]);
}

#[test]
fn favorite_ensemble_member_phase_offsets_are_evenly_distributed() {
    assert_eq!(member_phase_offset_ratio(0, 1), 0.0);
    assert_eq!(member_phase_offset_ratio(0, 3), 0.0);
    assert_eq!(member_phase_offset_ratio(1, 3), 1.0 / 3.0);
    assert_eq!(member_phase_offset_ratio(2, 3), 2.0 / 3.0);
}

#[test]
fn favorite_ensemble_member_eye_blink_elapsed_is_staggered() {
    assert_eq!(member_eye_blink_elapsed(0, 1), Duration::ZERO);
    assert_eq!(member_eye_blink_elapsed(0, 3), Duration::ZERO);
    assert_eq!(
        member_eye_blink_elapsed(1, 3),
        Duration::from_secs_f32(1.0 / 3.0)
    );
    assert_eq!(
        member_eye_blink_elapsed(2, 3),
        Duration::from_secs_f32(2.0 / 3.0)
    );
}

#[test]
fn favorite_ensemble_member_eye_blink_seeds_are_distinct_per_member() {
    assert_ne!(member_eye_blink_seed(0, 3), member_eye_blink_seed(1, 3));
    assert_ne!(member_eye_blink_seed(1, 3), member_eye_blink_seed(2, 3));
}

#[test]
fn favorite_ensemble_sanitize_deduplicates_equivalent_visibility_overrides() {
    let sanitized = sanitize_favorites_for_test(vec![
        FavoriteEnsembleEntry {
            zip_path: PathBuf::from("dummy-a.zip"),
            psd_path_in_zip: PathBuf::from("dummy/body.psd"),
            psd_file_name: "body.psd".to_string(),
            visibility_overrides: vec![
                LayerVisibilityOverride {
                    layer_index: 1,
                    visible: true,
                },
                LayerVisibilityOverride {
                    layer_index: 3,
                    visible: false,
                },
            ],
            mascot_scale: Some(1.0),
            favorite_ensemble_position: Some([10.0, 20.0]),
        },
        FavoriteEnsembleEntry {
            zip_path: PathBuf::from("dummy-a.zip"),
            psd_path_in_zip: PathBuf::from("dummy/body.psd"),
            psd_file_name: "body.psd".to_string(),
            visibility_overrides: vec![
                LayerVisibilityOverride {
                    layer_index: 1,
                    visible: true,
                },
                LayerVisibilityOverride {
                    layer_index: 3,
                    visible: false,
                },
            ],
            mascot_scale: Some(2.0),
            favorite_ensemble_position: Some([30.0, 40.0]),
        },
    ]);

    assert_eq!(sanitized.len(), 1);
    assert_eq!(sanitized[0].mascot_scale, Some(2.0));
    assert_eq!(sanitized[0].favorite_ensemble_position, Some([30.0, 40.0]));
}

fn sample_favorite_entry(mascot_scale: Option<f32>) -> FavoriteEnsembleEntry {
    FavoriteEnsembleEntry {
        zip_path: PathBuf::from("dummy-a.zip"),
        psd_path_in_zip: PathBuf::from("dummy/body.psd"),
        psd_file_name: "body.psd".to_string(),
        visibility_overrides: Vec::new(),
        mascot_scale,
        favorite_ensemble_position: None,
    }
}

fn rgba_with_alpha(alpha_values: &[u8]) -> Vec<u8> {
    alpha_values
        .iter()
        .flat_map(|&alpha| [0, 0, 0, alpha])
        .collect()
}
