use crate::render::blend_pixel;

#[test]
fn blend_pixel_over_transparent_background() {
    let out = blend_pixel([0, 0, 0, 0], [255, 0, 0, 255], 1.0);
    assert_eq!(out, [255, 0, 0, 255]);
}

#[test]
fn blend_pixel_respects_opacity() {
    let out = blend_pixel([0, 0, 255, 255], [255, 0, 0, 255], 0.5);
    assert!(out[0] > 0);
    assert!(out[2] > 0);
    assert_eq!(out[3], 255);
}
