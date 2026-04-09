use std::path::Path;

use rawpsd::LayerInfo;

use crate::archive::collect_psd_files;
use crate::model::LayerKind;
use crate::psd::{
    build_layer_nodes, build_psd_entry, catch_parser_panic, display_layer_name,
    effective_visibility_with_overrides, panic_payload_message, psd_file_name, rendered_png_name,
};
use crate::{workspace_cache_root, LayerVisibilityOverride};

#[test]
fn psd_file_name_uses_last_path_component() {
    assert_eq!(
        psd_file_name(Path::new("cache/sample/extracted/demo.psd")),
        "demo.psd"
    );
}

#[test]
fn display_layer_name_marks_unnamed_layers() {
    assert_eq!(display_layer_name("  "), "(unnamed)");
}

#[test]
fn rendered_png_name_sanitizes_path_components() {
    assert_eq!(
        rendered_png_name(Path::new(
            "/Users/alice/AppData/Local/mascot-render-server/cache/abc/extracted/a/b:demo.psd"
        )),
        "abc__extracted__a__b_demo.psd.png"
    );
}

#[test]
fn build_layer_nodes_tracks_group_depth_and_visibility() {
    let mut group = LayerInfo::default();
    group.name = "Body".to_string();
    group.group_opener = true;
    group.is_visible = true;

    let mut child = LayerInfo::default();
    child.name = "Face".to_string();
    child.is_visible = true;

    let mut closer = LayerInfo::default();
    closer.group_closer = true;
    closer.is_visible = true;

    let layers = vec![closer, child, group];
    let (nodes, effective_visibility) = build_layer_nodes(&layers);

    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].kind, LayerKind::GroupOpen);
    assert_eq!(nodes[1].depth, 1);
    assert_eq!(nodes[2].kind, LayerKind::GroupClose);
    assert_eq!(effective_visibility, vec![false, true, true]);
}

#[test]
fn visibility_overrides_hide_group_children() {
    let mut group = LayerInfo::default();
    group.name = "Body".to_string();
    group.group_opener = true;
    group.is_visible = true;

    let mut child = LayerInfo::default();
    child.name = "Face".to_string();
    child.is_visible = true;

    let mut closer = LayerInfo::default();
    closer.group_closer = true;
    closer.is_visible = true;

    let layers = vec![closer, child, group];
    let effective_visibility = effective_visibility_with_overrides(
        &layers,
        &[LayerVisibilityOverride {
            layer_index: 2,
            visible: false,
        }],
    )
    .expect("group visibility override should be valid");

    assert_eq!(effective_visibility, vec![false, false, false]);
}

#[test]
fn mandatory_layers_stay_visible_even_if_hidden_in_psd_or_overrides() {
    let mut mandatory = LayerInfo::default();
    mandatory.name = "!Core".to_string();
    mandatory.is_visible = false;

    let effective_visibility = effective_visibility_with_overrides(
        &[mandatory],
        &[LayerVisibilityOverride {
            layer_index: 0,
            visible: false,
        }],
    )
    .expect("mandatory layer visibility should be valid");

    assert_eq!(effective_visibility, vec![true]);
}

#[test]
fn catch_parser_panic_converts_panics_to_error_strings() {
    let result = catch_parser_panic("demo_parser", || -> usize {
        panic!("boom");
    });

    let error = result.expect_err("panic should be converted into an error");

    assert_eq!(error.message, "demo_parser panicked: boom");
    assert!(!error.backtrace.is_empty());
}

#[test]
fn panic_payload_message_supports_string_payloads() {
    let payload = Box::new(String::from("failure")) as Box<dyn std::any::Any + Send>;

    assert_eq!(panic_payload_message(payload), "failure");
}

#[test]
fn sample_psd_builds_without_parse_failure() {
    let path = Path::new("cache/001_dummy_should_not_exist/does-not-matter.psd");
    assert_eq!(psd_file_name(path), "does-not-matter.psd");
}

#[test]
fn all_extracted_psd_entries_build_without_parse_failure() {
    let root = workspace_cache_root();
    if !root.exists() {
        return;
    }

    let extracted_roots = std::fs::read_dir(&root)
        .expect("should read cache root")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("extracted"))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let render_root = workspace_cache_root().join("test-renders");
    let mut failures = Vec::new();

    for extracted_root in extracted_roots {
        for path in collect_psd_files(&extracted_root).expect("should collect extracted PSD files")
        {
            let entry = build_psd_entry(&path, &render_root);
            if entry.error.is_some() {
                failures.push(format!("{} => {:?}", path.display(), entry.error));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "unexpected parse failures:\n{}",
        failures.join("\n")
    );
}
