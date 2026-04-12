use std::fs;
use std::path::Path;

use crate::rgba_cache::{
    load_rgba_cache, rgba_cache_data_path, rgba_cache_meta_path, write_default_rgba_cache_for_rgba,
};
use crate::{load_mascot_image_with_report, workspace_cache_root};

#[test]
fn load_mascot_image_uses_matching_raw_rgba_cache_without_png_decode() {
    let root = workspace_cache_root().join("test-raw-rgba-cache-hit");
    let _ = fs::remove_dir_all(&root);
    let png_path = root.join("default.png");
    write_file(&png_path, b"not-a-real-png");
    let rgba = vec![255, 0, 0, 255, 0, 255, 0, 128];
    write_default_rgba_cache_for_rgba(&png_path, [2, 1], &rgba).unwrap();

    let (image, report) = load_mascot_image_with_report(&png_path).unwrap();

    assert_eq!(image.width, 2);
    assert_eq!(image.height, 1);
    assert_eq!(image.rgba, rgba);
    assert!(report.raw_rgba_cache_hit);
    assert_eq!(report.raw_rgba_cache_status, "hit");
    assert_eq!(report.read_file_ms, 0);
    assert_eq!(report.decode_png_ms, 0);
    assert!(rgba_cache_meta_path(&png_path).exists());
    assert!(rgba_cache_data_path(&png_path).exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn raw_rgba_cache_rejects_stale_png_source() {
    let root = workspace_cache_root().join("test-raw-rgba-cache-stale");
    let _ = fs::remove_dir_all(&root);
    let png_path = root.join("default.png");
    write_file(&png_path, b"png-v1");
    write_default_rgba_cache_for_rgba(&png_path, [1, 1], &[0, 0, 0, 0]).unwrap();
    write_file(&png_path, b"png-v2-changed");

    let (cached, report) = load_rgba_cache(&png_path).unwrap();

    assert!(cached.is_none());
    assert_eq!(report.status, "stale");

    let _ = fs::remove_dir_all(&root);
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}
