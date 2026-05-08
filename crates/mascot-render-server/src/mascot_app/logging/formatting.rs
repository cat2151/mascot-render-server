use eframe::egui::{Pos2, Vec2};
use mascot_render_protocol::{PlacementAnchorPositions, VisualSizePx};

pub(super) fn scale_text(value: f32) -> String {
    format!("{value:.3}")
}

pub(super) fn format_vec2(value: Vec2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

pub(super) fn optional_pos2_text(value: Option<Pos2>) -> String {
    value.map(format_pos2).unwrap_or_else(|| "-".to_string())
}

pub(super) fn optional_vec2_text(value: Option<Vec2>) -> String {
    value.map(format_vec2).unwrap_or_else(|| "-".to_string())
}

pub(super) fn optional_scale_text(value: Option<f32>) -> String {
    value.map(scale_text).unwrap_or_else(|| "-".to_string())
}

pub(super) fn optional_visual_size_text(value: Option<VisualSizePx>) -> String {
    value
        .map(|size| format!("{:.3}x{:.3}", size.width, size.height))
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn optional_anchor_positions_text(value: Option<PlacementAnchorPositions>) -> String {
    value
        .map(|positions| {
            format!(
                "bottom_center:{}|bottom_right:{}",
                format_pair(positions.bottom_center),
                format_pair(positions.bottom_right)
            )
        })
        .unwrap_or_else(|| "-".to_string())
}

fn format_pos2(value: Pos2) -> String {
    format!("{:.3},{:.3}", value.x, value.y)
}

fn format_pair([x, y]: [f32; 2]) -> String {
    format!("{x:.3},{y:.3}")
}
