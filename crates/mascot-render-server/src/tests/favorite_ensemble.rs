use crate::favorite_ensemble::{
    fill_missing_positions, pack_positions_from_right, patch_favorite_ensemble_positions_toml,
    FavoriteEnsembleEntry, FavoriteEnsembleLayoutEntry,
};
use mascot_render_core::LayerVisibilityOverride;
use std::path::PathBuf;

#[test]
fn favorite_ensemble_packs_entries_from_right_edge_without_horizontal_gaps() {
    let positions = pack_positions_from_right(&[[80.0, 120.0], [40.0, 60.0], [30.0, 90.0]]);

    assert_eq!(positions.len(), 3);
    assert_eq!(
        positions[0],
        [70.0, 0.0],
        "first favorite should sit at the right edge"
    );
    assert_eq!(
        positions[1],
        [30.0, 60.0],
        "second favorite should continue to the left"
    );
    assert_eq!(
        positions[2],
        [0.0, 30.0],
        "later favorites should keep filling leftward"
    );
}

#[test]
fn favorite_ensemble_only_places_missing_entries_to_the_left_of_existing_layout() {
    let mut layout = vec![
        FavoriteEnsembleLayoutEntry {
            size: [80.0, 120.0],
            position: Some([70.0, 0.0]),
        },
        FavoriteEnsembleLayoutEntry {
            size: [40.0, 60.0],
            position: None,
        },
        FavoriteEnsembleLayoutEntry {
            size: [30.0, 90.0],
            position: Some([40.0, 30.0]),
        },
    ];

    let updated = fill_missing_positions(&mut layout);

    assert_eq!(updated, vec![1]);
    assert_eq!(layout[0].position, Some([70.0, 0.0]));
    assert_eq!(layout[1].position, Some([0.0, 60.0]));
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
