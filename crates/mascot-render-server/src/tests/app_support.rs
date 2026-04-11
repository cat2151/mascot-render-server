use std::path::PathBuf;

use mascot_render_core::MascotImageData;

use crate::app_support::cached_skin_from_image;

#[test]
fn cached_skin_from_image_accepts_taller_image_before_backend_texture_limit_is_known() {
    const HEIGHT: usize = 2049;

    let ctx = eframe::egui::Context::default();
    assert_eq!(ctx.input(|input| input.max_texture_side), 2048);

    let image = MascotImageData {
        path: PathBuf::from("cache/demo/tall.png"),
        width: 1,
        height: HEIGHT as u32,
        rgba: vec![255; HEIGHT * 4],
    };

    let skin = cached_skin_from_image(&ctx, &image);

    assert_eq!(skin.image_size, [1, HEIGHT as u32]);
}
