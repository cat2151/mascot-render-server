#[cfg(target_os = "windows")]
use std::sync::Arc;

#[cfg(target_os = "windows")]
use eframe::egui::{Pos2, Rect};

#[cfg(target_os = "windows")]
use crate::window_region::{
    build_opaque_region_rects, build_window_region_data, PhysicalRect, WindowRegionCache,
    WindowRegionKey,
};

#[cfg(target_os = "windows")]
#[test]
fn opaque_runs_expand_to_window_regions() {
    let rects = build_opaque_region_rects(
        [4, 2],
        Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 4.0)),
        1.0,
        &[0, 255, 255, 0, 255, 0, 0, 255],
        8,
    )
    .unwrap();

    assert_eq!(
        rects,
        vec![
            PhysicalRect {
                left: 2,
                top: 0,
                right: 6,
                bottom: 2,
            },
            PhysicalRect {
                left: 0,
                top: 2,
                right: 2,
                bottom: 4,
            },
            PhysicalRect {
                left: 6,
                top: 2,
                right: 8,
                bottom: 4,
            },
        ]
    );
}

#[cfg(target_os = "windows")]
#[test]
fn fractional_image_offsets_round_outside_the_transparent_gap() {
    let rects = build_opaque_region_rects(
        [2, 2],
        Rect::from_min_max(Pos2::new(0.25, 0.5), Pos2::new(4.25, 4.5)),
        1.0,
        &[255, 0, 0, 255],
        8,
    )
    .unwrap();

    assert_eq!(
        rects,
        vec![
            PhysicalRect {
                left: 1,
                top: 1,
                right: 3,
                bottom: 3,
            },
            PhysicalRect {
                left: 3,
                top: 3,
                right: 5,
                bottom: 5,
            },
        ]
    );
}

#[cfg(target_os = "windows")]
#[test]
fn vertically_adjacent_runs_merge_into_taller_regions() {
    let rects = build_opaque_region_rects(
        [4, 3],
        Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 6.0)),
        1.0,
        &[
            0, 255, 255, 0, //
            0, 255, 255, 0, //
            0, 255, 255, 0, //
        ],
        8,
    )
    .unwrap();

    assert_eq!(
        rects,
        vec![PhysicalRect {
            left: 2,
            top: 0,
            right: 6,
            bottom: 6,
        }]
    );
}

#[cfg(target_os = "windows")]
#[test]
fn same_region_keeps_same_signature_across_blink_masks() {
    let open = build_window_region_data(
        [4, 2],
        Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 4.0)),
        1.0,
        &[0, 255, 255, 0, 255, 255, 255, 0],
        8,
    )
    .unwrap();
    let blink = build_window_region_data(
        [4, 2],
        Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 4.0)),
        1.0,
        &[0, 255, 255, 0, 255, 200, 255, 0],
        8,
    )
    .unwrap();

    assert_eq!(open.rects, blink.rects);
    assert_eq!(open.signature, blink.signature);
}

#[cfg(target_os = "windows")]
#[test]
fn region_cache_reuses_open_and_blink_entries() {
    let image_size = [4, 2];
    let image_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(8.0, 4.0));
    let open_mask = [0, 255, 255, 0, 255, 255, 255, 0];
    let blink_mask = [0, 255, 255, 0, 255, 200, 255, 0];
    let open_key = WindowRegionKey::new(true, image_size, image_rect, 1.0, &open_mask);
    let blink_key = WindowRegionKey::new(true, image_size, image_rect, 1.0, &blink_mask);
    let mut cache = WindowRegionCache::new();

    let open1 = cache
        .data_for(open_key, image_size, image_rect, 1.0, &open_mask, 8)
        .unwrap();
    let blink1 = cache
        .data_for(blink_key, image_size, image_rect, 1.0, &blink_mask, 8)
        .unwrap();
    let open2 = cache
        .data_for(open_key, image_size, image_rect, 1.0, &open_mask, 8)
        .unwrap();
    let blink2 = cache
        .data_for(blink_key, image_size, image_rect, 1.0, &blink_mask, 8)
        .unwrap();

    assert!(Arc::ptr_eq(&open1, &open2));
    assert!(Arc::ptr_eq(&blink1, &blink2));
}
