use std::fs;
use std::path::PathBuf;

use crate::{
    load_variation_spec, save_variation_spec, variation_hash, variation_png_path,
    variation_render_meta_path, variation_spec_path, workspace_cache_root, workspace_path,
    DisplayDiff, LayerVisibilityOverride,
};

#[test]
fn variation_paths_live_under_variations_directory() {
    let zip_cache_dir = workspace_cache_root().join("test-variation-layout");
    let psd_path_in_zip = PathBuf::from("demo/basic.psd");
    let mut variation = DisplayDiff::new();
    variation
        .visibility_overrides
        .push(LayerVisibilityOverride {
            layer_index: 3,
            visible: false,
        });

    let png_path = variation_png_path(&zip_cache_dir, &psd_path_in_zip, "basic.psd", &variation);

    assert!(
        png_path.to_string_lossy().contains("variations"),
        "variation PNG should live under variations/: {}",
        png_path.display()
    );
    assert_eq!(
        png_path.file_stem().and_then(|value| value.to_str()),
        Some(variation_hash(&variation).as_str())
    );
    assert_eq!(
        variation_spec_path(&png_path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("json")
    );
    assert_eq!(
        variation_render_meta_path(&png_path)
            .file_name()
            .and_then(|value| value.to_str()),
        Some(format!("{}.render.json", variation_hash(&variation)).as_str())
    );
}

#[test]
fn saved_variation_spec_round_trips_for_matching_psd() {
    let cache_dir = workspace_cache_root().join("test-variation-save");
    let _ = fs::remove_dir_all(&cache_dir);
    let zip_path = workspace_path("assets/zip/demo.zip");
    let psd_path_in_zip = PathBuf::from("demo/basic.psd");
    let mut variation = DisplayDiff::new();
    variation
        .visibility_overrides
        .push(LayerVisibilityOverride {
            layer_index: 5,
            visible: true,
        });

    let png_path = variation_png_path(&cache_dir, &psd_path_in_zip, "basic.psd", &variation);
    let spec_path = variation_spec_path(&png_path);
    save_variation_spec(&spec_path, &zip_path, &psd_path_in_zip, &variation)
        .expect("should save variation spec");

    let loaded = load_variation_spec(&spec_path, &zip_path, &psd_path_in_zip)
        .expect("should load matching variation spec");

    assert_eq!(loaded, variation);
}
