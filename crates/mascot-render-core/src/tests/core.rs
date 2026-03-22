use std::fs;
use std::path::PathBuf;

use crate::{
    variation_spec_path, workspace_cache_root, workspace_path, Core, CoreConfig, DisplayDiff,
    LayerVisibilityOverride, RenderRequest,
};

fn sample_zip_path() -> Option<PathBuf> {
    let candidates = [
        workspace_path("assets/zip/ずんだもん立ち絵素材2.3.zip"),
        workspace_path("assets/zip/ずんだもん立ち絵素材V3.2.zip"),
        workspace_path("assets/zip/ずんだもん立ち絵素材改1.1.1.zip"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn test_core(test_name: &str) -> Option<Core> {
    let cache_dir = workspace_cache_root().join(test_name);
    let _ = fs::remove_dir_all(&cache_dir);

    sample_zip_path().map(|_| Core::new(CoreConfig { cache_dir }))
}

#[test]
fn inspect_psd_returns_layer_descriptors_for_sample_zip() {
    let Some(core) = test_core("test-core-api-inspect") else {
        return;
    };
    let zip_path = sample_zip_path().expect("sample zip path should exist");

    let psds = core.list_psds(&zip_path).expect("should list PSDs");
    assert!(!psds.is_empty(), "expected at least one PSD in sample zip");

    let document = core
        .inspect_psd(&zip_path, &psds[0].path_in_zip)
        .expect("should inspect PSD");

    assert_eq!(document.zip_path, zip_path);
    assert_eq!(document.psd_path_in_zip, psds[0].path_in_zip);
    assert_eq!(document.file_name, psds[0].file_name);
    assert!(
        !document.layers.is_empty(),
        "expected inspected PSD to expose layer descriptors"
    );
    assert!(document.error.is_none(), "unexpected PSD parse error");
}

#[test]
fn render_png_uses_default_render_for_default_variation() {
    let Some(core) = test_core("test-core-api-render-default") else {
        return;
    };
    let zip_path = sample_zip_path().expect("sample zip path should exist");
    let psd = core
        .list_psds(&zip_path)
        .expect("should list PSDs")
        .into_iter()
        .next()
        .expect("expected at least one PSD");

    let rendered = core
        .render_png(RenderRequest {
            zip_path,
            psd_path_in_zip: psd.path_in_zip,
            display_diff: DisplayDiff::new(),
        })
        .expect("default render should succeed");

    assert_eq!(
        Some(rendered.output_path.clone()),
        psd.default_rendered_png_path,
        "default variation should reuse the default render cache"
    );
    assert!(
        rendered.cache_hit,
        "default variation should be treated as cache hit"
    );
}

#[test]
fn render_png_reuses_cached_custom_render() {
    let Some(core) = test_core("test-core-api-render") else {
        return;
    };
    let zip_path = sample_zip_path().expect("sample zip path should exist");
    let psd = core
        .list_psds(&zip_path)
        .expect("should list PSDs")
        .into_iter()
        .next()
        .expect("expected at least one PSD");
    let document = core
        .inspect_psd(&zip_path, &psd.path_in_zip)
        .expect("should inspect PSD");
    let mut display_diff = DisplayDiff::new();

    if let Some(layer) = document.layers.first() {
        display_diff
            .visibility_overrides
            .push(LayerVisibilityOverride {
                layer_index: layer.layer_index,
                visible: !layer.default_visible,
            });
    }

    let request = RenderRequest {
        zip_path: zip_path.clone(),
        psd_path_in_zip: psd.path_in_zip,
        display_diff,
    };

    let first = core
        .render_png(request.clone())
        .expect("first render should succeed");
    let second = core
        .render_png(request)
        .expect("second render should succeed");

    assert!(first.output_path.exists(), "rendered PNG should exist");
    assert!(!first.cache_hit, "first render should be a cache miss");
    assert!(second.cache_hit, "second render should hit the cache");
    assert_eq!(first.output_path, second.output_path);
    assert!(
        first.output_path.to_string_lossy().contains("variations"),
        "variation render should live under variations/: {}",
        first.output_path.display()
    );
    assert!(
        variation_spec_path(&first.output_path).exists(),
        "variation spec sidecar should exist"
    );
}
