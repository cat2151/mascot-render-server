use std::time::Duration;

use eframe::egui::{pos2, Rect, TextureId};

use crate::always_bend;

#[test]
fn sample_always_bend_swings_left_and_right() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 480.0));

    let resting = always_bend::sample_always_bend(Duration::ZERO, image_rect);
    let right = always_bend::sample_always_bend(Duration::from_millis(1_050), image_rect);
    let left = always_bend::sample_always_bend(Duration::from_millis(3_150), image_rect);

    assert_eq!(resting.offset_x, 0.0);
    assert!(right.offset_x > 0.0);
    assert!(left.offset_x < 0.0);
    assert!(right.offset_x.abs() <= image_rect.width() * 0.03);
}

#[test]
fn always_bend_mesh_moves_upper_center_more_than_lower_center() {
    let image_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(120.0, 240.0));
    let bend = always_bend::sample_always_bend(Duration::from_millis(1_050), image_rect);
    let mesh = always_bend::mesh(TextureId::Managed(7), image_rect, bend);

    let stride = 5;
    let top_center = mesh.vertices[2].pos.x;
    let middle_center = mesh.vertices[(6 * stride) + 2].pos.x;
    let bottom_center = mesh.vertices[(12 * stride) + 2].pos.x;
    let edge_x = mesh.vertices[0].pos.x;

    assert!(top_center > middle_center);
    assert!(middle_center > bottom_center);
    assert_eq!(bottom_center, image_rect.center().x);
    assert_eq!(edge_x, image_rect.left());
}
