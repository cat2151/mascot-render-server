use anyhow::Result;
use eframe::egui::{Pos2, Rect};
use eframe::CreationContext;
use std::time::{Duration, Instant};

const TRANSPARENT_INPUT_DEBUG_FLASH_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy)]
pub struct TransparentHitTestUpdate {
    pub now: Instant,
}

#[derive(Default)]
pub struct TransparentHitTestWindow {
    transparent_input_visual_until: Option<Instant>,
}

impl TransparentHitTestWindow {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn try_install(_cc: &CreationContext<'_>) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn update(&mut self, update: TransparentHitTestUpdate) {
        if self
            .transparent_input_visual_until
            .is_some_and(|until| until <= update.now)
        {
            self.transparent_input_visual_until = None;
        }
    }

    pub fn transparent_input_visual_remaining(&self, now: Instant) -> Option<Duration> {
        self.transparent_input_visual_until
            .and_then(|until| until.checked_duration_since(now))
    }

    pub fn flash_transparent_input_visual(&mut self) {
        self.transparent_input_visual_until =
            Some(Instant::now() + TRANSPARENT_INPUT_DEBUG_FLASH_DURATION);
    }
}

#[cfg(test)]
pub(crate) fn captures_client_point(
    image_size: [u32; 2],
    image_rect: Rect,
    pixels_per_point: f32,
    alpha_mask: &[u8],
    client_point: [i32; 2],
    alpha_threshold: u8,
) -> bool {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return false;
    }
    if alpha_mask.len() != (width as usize * height as usize) {
        return false;
    }
    if image_rect.width() <= 0.0 || image_rect.height() <= 0.0 || pixels_per_point <= 0.0 {
        return false;
    }
    let logical_point = Pos2::new(
        client_point[0] as f32 / pixels_per_point,
        client_point[1] as f32 / pixels_per_point,
    );
    captures_logical_point(
        image_size,
        image_rect,
        alpha_mask,
        logical_point,
        alpha_threshold,
    )
}

pub fn captures_logical_point(
    image_size: [u32; 2],
    image_rect: Rect,
    alpha_mask: &[u8],
    logical_point: Pos2,
    alpha_threshold: u8,
) -> bool {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return false;
    }
    if alpha_mask.len() != (width as usize * height as usize) {
        return false;
    }
    if image_rect.width() <= 0.0 || image_rect.height() <= 0.0 {
        return false;
    }
    if !image_rect.contains(logical_point) {
        return false;
    }
    let image_x =
        image_index_for_axis(logical_point.x, image_rect.min.x, image_rect.width(), width);
    let image_y = image_index_for_axis(
        logical_point.y,
        image_rect.min.y,
        image_rect.height(),
        height,
    );
    let alpha_index = image_y as usize * width as usize + image_x as usize;
    alpha_mask[alpha_index] > alpha_threshold
}

fn image_index_for_axis(position: f32, min: f32, span: f32, dimension: u32) -> u32 {
    let max_index = dimension.saturating_sub(1);
    let normalized = ((position - min) / span).clamp(0.0, 0.999_999_94);
    ((normalized * dimension as f32).floor() as u32).min(max_index)
}
