use eframe::egui::{Pos2, Rect, Vec2};
use mascot_render_core::{BounceAnimationConfig, MotionTransform, SquashBounceAnimationConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl AlphaBounds {
    pub fn full(image_size: [u32; 2]) -> Self {
        Self {
            min_x: 0,
            min_y: 0,
            max_x: image_size[0],
            max_y: image_size[1],
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

pub fn alpha_bounds_from_mask(
    image_size: [u32; 2],
    alpha_mask: &[u8],
    alpha_threshold: u8,
) -> Option<AlphaBounds> {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return None;
    }
    if alpha_mask.len() != (width as usize * height as usize) {
        return None;
    }

    let threshold = alpha_threshold.max(1);
    let width_usize = width as usize;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for (index, &alpha) in alpha_mask.iter().enumerate() {
        if alpha < threshold {
            continue;
        }

        let x = (index % width_usize) as u32;
        let y = (index / width_usize) as u32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + 1);
        max_y = max_y.max(y + 1);
        found = true;
    }

    found.then_some(AlphaBounds {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MascotWindowLayout {
    crop_rect: Rect,
    shake_amplitude_px: f32,
}

impl MascotWindowLayout {
    pub fn full(base_size: Vec2) -> Self {
        Self {
            crop_rect: Rect::from_min_size(Pos2::ZERO, base_size),
            shake_amplitude_px: 0.0,
        }
    }

    pub fn new(
        base_size: Vec2,
        image_size: [u32; 2],
        content_bounds: AlphaBounds,
        bounce: BounceAnimationConfig,
        squash_bounce: SquashBounceAnimationConfig,
    ) -> Self {
        let (crop_rect, shake_amplitude_px) =
            crop_rect_for_motion(base_size, image_size, content_bounds, bounce, squash_bounce);
        Self {
            crop_rect,
            shake_amplitude_px,
        }
    }

    pub fn window_size(self) -> Vec2 {
        self.crop_rect.size()
    }

    pub fn image_rect(self, base_size: Vec2, transform: MotionTransform) -> Rect {
        transformed_image_rect(base_size, transform).translate(-self.crop_rect.min.to_vec2())
    }

    pub fn canvas_origin_offset(self, base_size: Vec2) -> Vec2 {
        self.image_rect(base_size, MotionTransform::identity())
            .min
            .to_vec2()
    }

    pub fn shake_amplitude_px(self) -> f32 {
        self.shake_amplitude_px
    }
}

pub fn anchored_inner_origin(
    previous_inner_origin: Pos2,
    previous_layout: MascotWindowLayout,
    previous_base_size: Vec2,
    next_layout: MascotWindowLayout,
    next_base_size: Vec2,
) -> Pos2 {
    let canvas_origin =
        previous_inner_origin + previous_layout.canvas_origin_offset(previous_base_size);
    canvas_origin - next_layout.canvas_origin_offset(next_base_size)
}

pub fn transformed_image_rect(base_size: Vec2, transform: MotionTransform) -> Rect {
    let scale_x = transform.scale_x.max(0.1);
    let scale_y = transform.scale_y.max(0.1);
    let size = Vec2::new(base_size.x * scale_x, base_size.y * scale_y);
    let min = Pos2::new(
        (base_size.x - size.x) * 0.5 + transform.offset_x,
        base_size.y - size.y + transform.offset_y,
    );
    Rect::from_min_size(min, size)
}

fn crop_rect_for_motion(
    base_size: Vec2,
    image_size: [u32; 2],
    content_bounds: AlphaBounds,
    bounce: BounceAnimationConfig,
    squash_bounce: SquashBounceAnimationConfig,
) -> (Rect, f32) {
    let base_candidates = [
        MotionTransform::identity(),
        MotionTransform {
            offset_x: 0.0,
            offset_y: -bounce.amplitude_px.max(0.0),
            scale_x: 1.0,
            scale_y: 1.0,
        },
        MotionTransform {
            offset_x: 0.0,
            offset_y: -squash_bounce.amplitude_px.max(0.0),
            scale_x: 1.0,
            scale_y: 1.0,
        },
        MotionTransform {
            offset_x: 0.0,
            offset_y: 0.0,
            scale_x: 1.0 + squash_bounce.squash_amount.max(0.0),
            scale_y: 1.0 - squash_bounce.stretch_amount.max(0.0),
        },
    ];
    let Some(base_union) =
        union_content_rects(base_size, image_size, content_bounds, &base_candidates)
    else {
        return (Rect::from_min_size(Pos2::ZERO, base_size), 0.0);
    };
    let shake_amplitude_px = shake_amplitude_px(base_union.height());
    let shake_candidates = [
        MotionTransform {
            offset_x: -shake_amplitude_px,
            offset_y: -shake_amplitude_px,
            scale_x: 1.0,
            scale_y: 1.0,
        },
        MotionTransform {
            offset_x: -shake_amplitude_px,
            offset_y: shake_amplitude_px,
            scale_x: 1.0,
            scale_y: 1.0,
        },
        MotionTransform {
            offset_x: shake_amplitude_px,
            offset_y: -shake_amplitude_px,
            scale_x: 1.0,
            scale_y: 1.0,
        },
        MotionTransform {
            offset_x: shake_amplitude_px,
            offset_y: shake_amplitude_px,
            scale_x: 1.0,
            scale_y: 1.0,
        },
    ];
    let crop_rect = union_content_rects(base_size, image_size, content_bounds, &shake_candidates)
        .map(|shake_union| base_union.union(shake_union))
        .unwrap_or(base_union);

    (crop_rect, shake_amplitude_px)
}

fn union_content_rects(
    base_size: Vec2,
    image_size: [u32; 2],
    content_bounds: AlphaBounds,
    transforms: &[MotionTransform],
) -> Option<Rect> {
    let mut union: Option<Rect> = None;
    for &transform in transforms {
        let full_image_rect = transformed_image_rect(base_size, transform);
        let visible_rect = content_rect(full_image_rect, image_size, content_bounds);
        union = Some(match union {
            Some(current) => current.union(visible_rect),
            None => visible_rect,
        });
    }

    union
}

fn shake_amplitude_px(window_height: f32) -> f32 {
    window_height.max(0.0) * 0.1
}

fn content_rect(image_rect: Rect, image_size: [u32; 2], content_bounds: AlphaBounds) -> Rect {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return image_rect;
    }

    let width = width as f32;
    let height = height as f32;
    let min = Pos2::new(
        image_rect.min.x + image_rect.width() * (content_bounds.min_x as f32 / width),
        image_rect.min.y + image_rect.height() * (content_bounds.min_y as f32 / height),
    );
    let max = Pos2::new(
        image_rect.min.x + image_rect.width() * (content_bounds.max_x as f32 / width),
        image_rect.min.y + image_rect.height() * (content_bounds.max_y as f32 / height),
    );
    Rect::from_min_max(min, max)
}
