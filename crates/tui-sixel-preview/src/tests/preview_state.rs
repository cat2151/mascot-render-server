use std::path::Path;

use crate::PreviewState;

#[test]
fn request_sync_marks_preview_as_loading() {
    let mut preview = PreviewState::new();

    preview.request_sync(Some(Path::new("cache/demo/render.png")));

    assert!(preview.is_loading());
    assert_eq!(
        preview.status(),
        "Loading preview from cache...\nrender.png"
    );
}

#[test]
fn request_sync_none_clears_loading_state() {
    let mut preview = PreviewState::new();
    preview.request_sync(Some(Path::new("cache/demo/render.png")));

    preview.request_sync(None);

    assert!(!preview.is_loading());
    assert_eq!(preview.status(), "No cached PNG preview.");
}

#[test]
fn request_sync_same_target_is_ignored() {
    let mut preview = PreviewState::new();

    preview.request_sync(Some(Path::new("cache/demo/render.png")));
    preview.request_sync(Some(Path::new("cache/demo/render.png")));

    assert!(preview.is_loading());
    assert_eq!(
        preview.status(),
        "Loading preview from cache...\nrender.png"
    );
}

#[test]
fn loading_overlay_stays_standard_without_sixel_cache() {
    let mut preview = PreviewState::new();

    preview.request_sync(Some(Path::new("cache/demo/render.png")));

    assert!(!preview.uses_compact_loading_overlay());
    assert_eq!(
        preview.loading_overlay_message(),
        "Loading preview from cache...\nrender.png"
    );
}

#[test]
fn loading_overlay_becomes_compact_when_sixel_cache_exists() {
    let mut preview = PreviewState::new();
    let png_path = Path::new("cache/demo/render.png");
    preview.cache_sixel_path_for_test(png_path);

    preview.request_sync(Some(png_path));

    assert!(preview.uses_compact_loading_overlay());
    assert_eq!(preview.loading_overlay_message(), "Loading preview...");
}

#[test]
fn has_sixel_cache_for_path_checks_target_path() {
    let mut preview = PreviewState::new();
    let png_path = Path::new("cache/demo/render.png");
    preview.cache_sixel_path_for_test(png_path);

    assert!(preview.has_sixel_cache_for_path(Some(png_path)));
    assert!(!preview.has_sixel_cache_for_path(Some(Path::new("cache/demo/other.png"))));
}
