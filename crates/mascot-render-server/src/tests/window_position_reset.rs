use eframe::egui::{Modifiers, Pos2, Vec2};
use mascot_render_protocol::ScreenRectPx;

use crate::mascot_app::{
    display_position_reset_requested_for_test, reset_outer_position_for_screen_for_test,
};

#[test]
fn display_position_reset_requires_focus_and_r_key() {
    assert!(display_position_reset_requested_for_test(
        true,
        Modifiers::NONE,
        true
    ));
    assert!(!display_position_reset_requested_for_test(
        false,
        Modifiers::NONE,
        true
    ));
    assert!(!display_position_reset_requested_for_test(
        true,
        Modifiers::NONE,
        false
    ));
}

#[test]
fn display_position_reset_ignores_system_modifier_combinations() {
    assert!(!display_position_reset_requested_for_test(
        true,
        Modifiers::ALT,
        true
    ));
    assert!(!display_position_reset_requested_for_test(
        true,
        Modifiers::CTRL,
        true
    ));
    assert!(display_position_reset_requested_for_test(
        true,
        Modifiers::SHIFT,
        true
    ));
}

#[test]
fn reset_outer_position_centers_window_in_screen_rect() {
    let position = reset_outer_position_for_screen_for_test(
        Vec2::new(200.0, 100.0),
        ScreenRectPx {
            min_x: 100.0,
            min_y: 50.0,
            max_x: 900.0,
            max_y: 650.0,
        },
    );

    assert_eq!(position, Some(Pos2::new(400.0, 300.0)));
}

#[test]
fn reset_outer_position_keeps_oversized_window_origin_visible() {
    let position = reset_outer_position_for_screen_for_test(
        Vec2::new(900.0, 700.0),
        ScreenRectPx {
            min_x: 100.0,
            min_y: 50.0,
            max_x: 900.0,
            max_y: 650.0,
        },
    );

    assert_eq!(position, Some(Pos2::new(100.0, 50.0)));
}

#[test]
fn reset_outer_position_rejects_invalid_screen_rect() {
    let position = reset_outer_position_for_screen_for_test(
        Vec2::new(200.0, 100.0),
        ScreenRectPx {
            min_x: 100.0,
            min_y: 50.0,
            max_x: 100.0,
            max_y: 650.0,
        },
    );

    assert_eq!(position, None);
}
