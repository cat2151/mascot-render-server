use std::path::PathBuf;

use eframe::egui::{Pos2, Vec2};
use mascot_render_protocol::{
    PlacementAnchorKind, PlacementAnchorPositions, PlacementMode, PlacementTargetScope,
    ScreenRectPx, VisualSizePx,
};

use crate::{
    anchor_positions_from_inner_origin, build_anchor_plan, clamp_zoomed_inner_origin_to_right_edge,
    shared_height_scale, visual_size_px, AlphaBounds, MascotWindowLayout, PlacementPlanTargetInput,
    PlacementState, PsdPlacementState,
};

#[test]
fn anchor_plan_uses_bottom_center_when_no_target_overflows_right() {
    let plan = build_anchor_plan(
        PlacementMode::PerPsd,
        PlacementTargetScope::CandidatePsdSet,
        screen_rect(1920.0),
        vec![target("a.psd", 1600.0, 200.0)],
    );

    assert_eq!(plan.selected_anchor_kind, PlacementAnchorKind::BottomCenter);
    assert_eq!(plan.max_right_overflow_px, 0.0);
    assert!(!plan.targets[0].overflows_right);
}

#[test]
fn anchor_plan_uses_bottom_right_when_any_target_overflows_right() {
    let plan = build_anchor_plan(
        PlacementMode::SharedVisualSize,
        PlacementTargetScope::CandidatePsdSet,
        screen_rect(1920.0),
        vec![
            target("ok.psd", 1600.0, 200.0),
            target("wide.psd", 1950.0, 200.0),
        ],
    );

    assert_eq!(plan.selected_anchor_kind, PlacementAnchorKind::BottomRight);
    assert_eq!(plan.max_right_overflow_px, 130.0);
    assert!(plan.targets[1].overflows_right);
}

#[test]
fn placement_state_sanitizes_invalid_values_and_keeps_last_duplicate() {
    let state = PlacementState {
        psd_states: vec![
            psd_state("demo.psd", Some(0.2), Some(visual(10.0, 20.0))),
            psd_state("demo.psd", Some(f32::NAN), Some(visual(-1.0, 20.0))),
        ],
        shared_visual_size_px: Some(visual(f32::INFINITY, 10.0)),
        shared_anchor_positions: Some(anchors([1.0, f32::NAN], [2.0, 3.0])),
        ..PlacementState::default()
    };

    let sanitized = state.sanitize().expect("version 1 should sanitize");

    assert_eq!(sanitized.psd_states.len(), 1);
    assert_eq!(sanitized.psd_states[0].scale, None);
    assert_eq!(sanitized.psd_states[0].visual_size_px, None);
    assert_eq!(sanitized.shared_visual_size_px, None);
    assert_eq!(sanitized.shared_anchor_positions, None);
}

#[test]
fn shared_visual_size_height_policy_uses_visible_height() {
    let scale = shared_height_scale(
        visual(320.0, 480.0),
        AlphaBounds {
            min_x: 10,
            min_y: 20,
            max_x: 110,
            max_y: 260,
        },
        [400, 800],
        0.01,
    )
    .expect("scale should be calculated");

    assert!((scale - 2.0).abs() < f32::EPSILON);
}

#[test]
fn visual_size_uses_alpha_bounds_and_scale() {
    let size = visual_size_px(
        AlphaBounds {
            min_x: 2,
            min_y: 3,
            max_x: 12,
            max_y: 23,
        },
        0.5,
    )
    .expect("positive finite scale should produce a visual size");

    assert_eq!(size, visual(5.0, 10.0));
}

#[test]
fn anchor_positions_include_bottom_center_and_bottom_right() {
    let layout = MascotWindowLayout::full(Vec2::new(100.0, 80.0));
    let positions = anchor_positions_from_inner_origin(Pos2::new(400.0, 300.0), layout);

    assert_eq!(positions.bottom_center, [450.0, 380.0]);
    assert_eq!(positions.bottom_right, [500.0, 380.0]);
}

#[test]
fn zoom_clamp_moves_bottom_center_zoom_back_inside_right_edge() {
    let clamped = clamp_zoomed_inner_origin_to_right_edge(
        Pos2::new(100.0, 300.0),
        MascotWindowLayout::full(Vec2::new(100.0, 80.0)),
        MascotWindowLayout::full(Vec2::new(200.0, 80.0)),
        PlacementAnchorKind::BottomCenter,
        screen_rect(210.0),
    )
    .expect("zoom should clamp when it newly overflows to the right");

    assert_eq!(clamped, Pos2::new(10.0, 300.0));
}

#[test]
fn zoom_clamp_ignores_positions_that_already_overflowed_before_zoom() {
    assert_eq!(
        clamp_zoomed_inner_origin_to_right_edge(
            Pos2::new(130.0, 300.0),
            MascotWindowLayout::full(Vec2::new(100.0, 80.0)),
            MascotWindowLayout::full(Vec2::new(200.0, 80.0)),
            PlacementAnchorKind::BottomCenter,
            screen_rect(210.0),
        ),
        None
    );
}

fn target(psd: &str, bottom_center_x: f32, visible_width: f32) -> PlacementPlanTargetInput {
    PlacementPlanTargetInput {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from(psd),
        scale: 1.0,
        visible_size_px: visual(visible_width, 300.0),
        bottom_center_anchor_position: [bottom_center_x, 900.0],
        bottom_right_anchor_position: [bottom_center_x + visible_width / 2.0, 900.0],
        bottom_center_anchor_offset: [visible_width / 2.0, 300.0],
        bottom_right_anchor_offset: [visible_width, 300.0],
    }
}

fn screen_rect(max_x: f32) -> ScreenRectPx {
    ScreenRectPx {
        min_x: 0.0,
        min_y: 0.0,
        max_x,
        max_y: 1080.0,
    }
}

fn psd_state(
    psd_path_in_zip: &str,
    scale: Option<f32>,
    visual_size_px: Option<VisualSizePx>,
) -> PsdPlacementState {
    PsdPlacementState {
        zip_path: PathBuf::from("demo.zip"),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        anchor_positions: Some(anchors([1.0, 2.0], [3.0, 4.0])),
        scale,
        visual_size_px,
        updated_at_unix_ms: 1,
    }
}

fn visual(width: f32, height: f32) -> VisualSizePx {
    VisualSizePx { width, height }
}

fn anchors(bottom_center: [f32; 2], bottom_right: [f32; 2]) -> PlacementAnchorPositions {
    PlacementAnchorPositions {
        bottom_center,
        bottom_right,
    }
}
