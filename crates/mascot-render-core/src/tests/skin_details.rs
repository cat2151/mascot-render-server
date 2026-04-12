use std::fs;
use std::path::Path;

use crate::{
    load_or_build_skin_details, skin_details_alpha_path, skin_details_meta_path,
    workspace_cache_root, MascotImageData, SkinContentBounds,
};

#[test]
fn skin_details_cache_reuses_alpha_mask_and_bounds_for_same_png_source() {
    let root = workspace_cache_root().join("test-skin-details-cache-reuse");
    let _ = fs::remove_dir_all(&root);
    let png_path = root.join("skin.png");
    write_file(&png_path, b"png-v1");

    let image = MascotImageData {
        path: png_path.clone(),
        width: 3,
        height: 2,
        rgba: rgba_from_alpha(&[0, 5, 0, 0, 9, 0]),
    };
    let (details, report) = load_or_build_skin_details(&image).unwrap();

    assert!(!report.cache_hit);
    assert_eq!(details.alpha_mask.as_ref(), &[0, 5, 0, 0, 9, 0]);
    assert_eq!(
        details.content_bounds,
        SkinContentBounds {
            min_x: 1,
            min_y: 0,
            max_x: 2,
            max_y: 2
        }
    );
    assert!(skin_details_meta_path(&png_path).exists());
    assert!(skin_details_alpha_path(&png_path).exists());

    let changed_rgba = MascotImageData {
        path: png_path.clone(),
        width: 3,
        height: 2,
        rgba: rgba_from_alpha(&[0, 0, 0, 255, 0, 0]),
    };
    let (cached_details, cached_report) = load_or_build_skin_details(&changed_rgba).unwrap();

    assert!(cached_report.cache_hit);
    assert_eq!(cached_details.alpha_mask.as_ref(), &[0, 5, 0, 0, 9, 0]);
    assert_eq!(cached_details.content_bounds, details.content_bounds);

    let _ = fs::remove_dir_all(&root);
}

fn rgba_from_alpha(alpha: &[u8]) -> Vec<u8> {
    alpha
        .iter()
        .flat_map(|alpha| [255, 255, 255, *alpha])
        .collect()
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}
