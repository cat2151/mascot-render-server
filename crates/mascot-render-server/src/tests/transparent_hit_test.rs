use eframe::egui::{Pos2, Rect};

use crate::transparent_hit_test::{captures_client_point, captures_logical_point};

fn alpha_mask(values: &[u8]) -> Vec<u8> {
    values.to_vec()
}

#[test]
fn transparent_pixels_pass_through() {
    let image_rect = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(14.0, 24.0));
    let alpha_mask = alpha_mask(&[
        255, 255, 255, 255, //
        255, 0, 0, 255, //
        255, 255, 255, 255, //
        255, 255, 255, 255, //
    ]);

    assert!(
        !captures_client_point([4, 4], image_rect, 1.0, &alpha_mask, [11, 21], 8),
        "fully transparent pixels should not block the background window"
    );
}

#[test]
fn opaque_pixels_capture_mouse() {
    let image_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(4.0, 4.0));
    let alpha_mask = alpha_mask(&[
        255, 255, 255, 255, //
        255, 255, 255, 255, //
        255, 255, 255, 255, //
        255, 255, 255, 255, //
    ]);

    assert!(
        captures_client_point([4, 4], image_rect, 1.0, &alpha_mask, [2, 2], 8),
        "opaque mascot pixels should still receive drag and click input"
    );
}

#[test]
fn physical_client_points_respect_pixels_per_point() {
    let image_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(4.0, 4.0));
    let alpha_mask = alpha_mask(&[
        255, 255, 255, 255, //
        255, 0, 255, 255, //
        255, 255, 255, 255, //
        255, 255, 255, 255, //
    ]);

    assert!(
        !captures_client_point([4, 4], image_rect, 2.0, &alpha_mask, [2, 2], 8),
        "DPI-scaled client coordinates should map to the same transparent pixel"
    );
    assert!(
        captures_client_point([4, 4], image_rect, 2.0, &alpha_mask, [6, 2], 8),
        "neighboring opaque pixels should still be clickable under DPI scaling"
    );
}

#[test]
fn points_outside_transformed_image_pass_through() {
    let image_rect = Rect::from_min_max(Pos2::new(50.0, 25.0), Pos2::new(150.0, 225.0));
    let alpha_mask = vec![255; 100 * 200];

    assert!(
        !captures_client_point([100, 200], image_rect, 1.0, &alpha_mask, [25, 25], 8),
        "areas outside the transformed mascot bounds should not intercept input"
    );
}

#[test]
fn logical_points_detect_transparent_pixels_without_client_conversion() {
    let image_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(4.0, 4.0));
    let alpha_mask = alpha_mask(&[
        255, 255, 255, 255, //
        255, 0, 255, 255, //
        255, 255, 255, 255, //
        255, 255, 255, 255, //
    ]);

    assert!(
        !captures_logical_point([4, 4], image_rect, &alpha_mask, Pos2::new(1.0, 1.0), 8),
        "transparent logical points should be treated as background input"
    );
    assert!(
        captures_logical_point([4, 4], image_rect, &alpha_mask, Pos2::new(2.0, 1.0), 8),
        "opaque logical points should still be treated as mascot input"
    );
}
