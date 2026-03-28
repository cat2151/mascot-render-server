use std::time::Duration;

use eframe::egui::{epaint::Vertex, Color32, Mesh, Pos2, Rect, TextureId};
use mascot_render_core::{AlwaysBendConfig, MotionTransform};

const ALWAYS_BEND_CYCLE: Duration = Duration::from_millis(4_200);
const ALWAYS_BEND_FRAME_INTERVAL: Duration = Duration::from_millis(16);
const ALWAYS_BEND_ROWS: usize = 12;
const ALWAYS_BEND_COLUMNS: usize = 4;

pub(crate) fn sample_always_bend(
    elapsed: Duration,
    image_rect: Rect,
    config: AlwaysBendConfig,
) -> MotionTransform {
    let cycle_nanos = ALWAYS_BEND_CYCLE.as_nanos();
    let elapsed_in_cycle = elapsed.as_nanos() % cycle_nanos;
    let phase = elapsed_in_cycle as f64 * std::f64::consts::TAU / cycle_nanos as f64;
    MotionTransform {
        offset_x: image_rect.width() * config.amplitude_ratio * phase.sin() as f32,
        ..MotionTransform::identity()
    }
}

pub(crate) fn repaint_after() -> Duration {
    ALWAYS_BEND_FRAME_INTERVAL
}

pub(crate) fn mesh(texture_id: TextureId, image_rect: Rect, bend: MotionTransform) -> Mesh {
    let mut mesh = Mesh {
        texture_id,
        ..Default::default()
    };

    for row in 0..=ALWAYS_BEND_ROWS {
        let v = normalized_step(row, ALWAYS_BEND_ROWS);
        let y = image_rect.top() + image_rect.height() * v;
        let row_bend = bend.offset_x * vertical_influence(v);

        for column in 0..=ALWAYS_BEND_COLUMNS {
            let u = normalized_step(column, ALWAYS_BEND_COLUMNS);
            let column_bend = row_bend * horizontal_influence(u);
            let x = image_rect.left() + image_rect.width() * u + column_bend;
            mesh.vertices.push(Vertex {
                pos: Pos2::new(x, y),
                uv: Pos2::new(u, v),
                color: Color32::WHITE,
            });
        }
    }

    let stride = ALWAYS_BEND_COLUMNS + 1;
    for row in 0..ALWAYS_BEND_ROWS {
        let row_start = row * stride;
        let next_row_start = row_start + stride;
        for column in 0..ALWAYS_BEND_COLUMNS {
            let top_left = (row_start + column) as u32;
            let top_right = top_left + 1;
            let bottom_left = (next_row_start + column) as u32;
            let bottom_right = bottom_left + 1;
            mesh.indices.extend_from_slice(&[
                top_left,
                top_right,
                bottom_left,
                top_right,
                bottom_right,
                bottom_left,
            ]);
        }
    }

    mesh
}

fn normalized_step(index: usize, max: usize) -> f32 {
    debug_assert!(max > 0, "bend mesh grid dimensions must stay positive");
    index as f32 / max.max(1) as f32
}

fn vertical_influence(v: f32) -> f32 {
    (1.0 - v).powi(2)
}

fn horizontal_influence(u: f32) -> f32 {
    let center_distance = (u - 0.5).abs() * 2.0;
    (1.0 - center_distance.powi(2)).max(0.0)
}
