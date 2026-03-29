use std::path::PathBuf;

use mascot_render_core::{
    auto_generate_eye_blink_target, DisplayDiff, LayerDescriptor, LayerKind, PsdDocument,
};

#[test]
fn automatic_eye_blink_generation_handles_generic_psds() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![
            descriptor(2, "*基本目", LayerKind::Layer, true, 0),
            descriptor(1, "*閉じ目", LayerKind::Layer, false, 0),
        ],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let target = auto_generate_eye_blink_target(&document, &DisplayDiff::default())
        .expect("should auto-generate");

    assert_eq!(target.psd_file_name, "demo.psd");
    assert_eq!(target.first_layer_name, "基本目");
    assert_eq!(target.second_layer_name, "閉じ目");
}

#[test]
fn automatic_eye_blink_generation_errors_without_closed_eye_candidates() {
    let document = PsdDocument {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from("demo.psd"),
        file_name: "demo.psd".to_string(),
        metadata: String::new(),
        layers: vec![descriptor(0, "*口", LayerKind::Layer, true, 0)],
        error: None,
        log_path: None,
        default_rendered_png_path: None,
        render_warnings: Vec::new(),
    };

    let error = auto_generate_eye_blink_target(&document, &DisplayDiff::default())
        .expect_err("should fail");

    assert!(error.contains("auto-detectable eye blink pair"));
    assert!(error.contains("閉じ目"));
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
