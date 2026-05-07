use std::path::Path;
use std::path::PathBuf;

use eframe::egui::{Pos2, Vec2};
use mascot_render_protocol::{
    PlacementAnchorKind, PlacementAnchorPositions, PlacementMode, VisualSizePx,
};

use crate::mascot_app::{
    change_character_failure_message_for_test, change_character_stage_message_for_test,
    change_character_success_message_for_test, clear_rendered_skin_path_for_test,
    hot_reload_context_message_for_test, record_rendered_skin_path_for_test,
    refresh_window_layout_message_for_test, reloaded_scale_message_for_test,
    rendered_skin_message_for_test, scale_change_message_for_test,
    scale_layout_change_message_for_test, should_log_rendered_skin_for_test,
    RefreshWindowLayoutDiagnosticsForTest, ScaleChangeTriggerForTest, ScaleLayoutChangeForTest,
};
use crate::MascotWindowLayout;
use mascot_render_server::window_history::ViewportInfo;

#[test]
fn change_character_stage_log_message_includes_stage_and_paths() {
    let message = change_character_stage_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        "load_base_skin",
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_character character変更を処理中です: stage=load_base_skin from=cache/anko/normal.png to=cache/zunda/normal.png"
    );
}

#[test]
fn change_character_success_log_message_reports_success() {
    let message = change_character_success_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        Path::new("config/mascot-render-server.runtime.json"),
        Path::new("cache/zunda/normal.png"),
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_character character変更に成功しました: from=cache/anko/normal.png to=cache/zunda/normal.png runtime_state_path=config/mascot-render-server.runtime.json persisted_png_path=cache/zunda/normal.png"
    );
}

#[test]
fn change_character_failure_log_message_reports_stage_and_error() {
    let message = change_character_failure_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        "refresh_mouth_flap_skins",
        "failed to refresh mouth-flap skins",
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_character character変更に失敗しました: stage=refresh_mouth_flap_skins from=cache/anko/normal.png to=cache/zunda/normal.png error=failed to refresh mouth-flap skins"
    );
}

#[test]
fn rendered_skin_log_message_includes_displayed_path_and_file_name() {
    let message = rendered_skin_message_for_test(Path::new("cache/shikoku/display.png"));

    assert_eq!(
        message,
        "trigger=render action=display_skin displayed_png_path=cache/shikoku/display.png displayed_png_file_name=display.png"
    );
}

#[test]
fn rendered_skin_log_state_skips_duplicate_paths_until_cleared() {
    let mut last_logged_skin_path = None::<PathBuf>;
    let displayed_path = Path::new("cache/shikoku/display.png");

    assert!(should_log_rendered_skin_for_test(
        last_logged_skin_path.as_deref(),
        displayed_path
    ));
    assert!(record_rendered_skin_path_for_test(
        &mut last_logged_skin_path,
        displayed_path
    ));
    assert_eq!(last_logged_skin_path.as_deref(), Some(displayed_path));
    assert!(!should_log_rendered_skin_for_test(
        last_logged_skin_path.as_deref(),
        displayed_path
    ));
    assert!(!record_rendered_skin_path_for_test(
        &mut last_logged_skin_path,
        displayed_path
    ));

    clear_rendered_skin_path_for_test(&mut last_logged_skin_path);

    assert!(should_log_rendered_skin_for_test(
        last_logged_skin_path.as_deref(),
        displayed_path
    ));
    assert!(record_rendered_skin_path_for_test(
        &mut last_logged_skin_path,
        displayed_path
    ));
    assert_eq!(last_logged_skin_path.as_deref(), Some(displayed_path));
}

#[test]
fn scale_change_log_message_reports_mouse_wheel_details() {
    let message = scale_change_message_for_test(
        ScaleChangeTriggerForTest::MouseWheel {
            raw_scroll_delta_y: 120.0,
        },
        1,
        1.0,
        1.1,
        false,
        Path::new("cache/zunda/normal.png"),
    );

    assert_eq!(
        message,
        "trigger=mouse_wheel action=change_scale scale変更を適用しました: steps=1 previous_scale=1.000 next_scale=1.100 raw_scroll_delta_y=120.000 favorite_ensemble_enabled=false configured_png_path=cache/zunda/normal.png"
    );
}

#[test]
fn scale_layout_change_log_message_reports_window_reposition_details() {
    let message = scale_layout_change_message_for_test(
        ScaleChangeTriggerForTest::MouseWheel {
            raw_scroll_delta_y: 120.0,
        },
        1,
        1.0,
        1.1,
        ScaleLayoutChangeForTest {
            selected_anchor_kind: PlacementAnchorKind::BottomCenter,
            previous_layout: MascotWindowLayout::full(Vec2::new(100.0, 200.0)),
            next_layout: MascotWindowLayout::full(Vec2::new(110.0, 220.0)),
            viewport_info: Some(ViewportInfo {
                inner_origin: Pos2::new(400.0, 300.0),
                inner_to_outer_offset: Vec2::new(8.0, 30.0),
            }),
        },
        Path::new("cache/zunda/normal.png"),
    );

    assert_eq!(
        message,
        "trigger=mouse_wheel action=change_scale_layout scale変更時のwindow再配置を計算しました: steps=1 previous_scale=1.000 next_scale=1.100 raw_scroll_delta_y=120.000 selected_anchor_kind=BottomCenter previous_window_size=100.000,200.000 next_window_size=110.000,220.000 previous_anchor_offset=50.000,200.000 next_anchor_offset=55.000,220.000 previous_inner_origin=400.000,300.000 previous_inner_to_outer_offset=8.000,30.000 next_inner_origin=395.000,280.000 next_outer_position=387.000,250.000 configured_png_path=cache/zunda/normal.png"
    );
}

#[test]
fn reloaded_scale_log_message_reports_revert_context() {
    let message = reloaded_scale_message_for_test(
        1.1,
        1.0,
        Some(1.1),
        Some(1.0),
        true,
        Path::new("config/mascot-render-server.runtime.json"),
        Path::new("config/mascot-render-server.toml"),
    );

    assert_eq!(
        message,
        "trigger=hot_reload action=change_scale scale変更を再読込しました: previous_scale=1.100 next_scale=1.000 previous_config_scale=1.100 reloaded_config_scale=1.000 pending_persisted_scale=true runtime_state_path=config/mascot-render-server.runtime.json config_path=config/mascot-render-server.toml"
    );
}

#[test]
fn hot_reload_context_log_message_reports_placement_inputs() {
    let message = hot_reload_context_message_for_test(
        false,
        true,
        false,
        true,
        false,
        true,
        true,
        false,
        false,
        true,
        true,
        PlacementMode::SharedVisualSize,
        PlacementAnchorKind::BottomCenter,
        Some(VisualSizePx {
            width: 208.08792,
            height: 457.3187,
        }),
        Some(PlacementAnchorPositions {
            bottom_center: [1557.1039, 818.07404],
            bottom_right: [1679.0143, 818.07404],
        }),
        Path::new("cache/old.png"),
        Path::new("cache/new.png"),
        Path::new("assets/old.zip"),
        Path::new("assets/new.zip"),
        Path::new("old/body.psd"),
        Path::new("new/body.psd"),
        Some(0.239),
        Some(0.145),
        Some(Pos2::new(1557.1039, 818.07404)),
    );

    assert_eq!(
        message,
        "trigger=hot_reload action=reload_config hot reload入力を検出しました: config_file_changed=false runtime_state_changed=true favorite_ensemble_file_changed=false psd_viewer_tui_activity_changed=true window_history_file_changed=false png_changed=true scale_changed=true favorite_ensemble_changed=false ensemble_mode_changed=false blink_source_changed=true history_path_changed=true placement_mode=SharedVisualSize selected_anchor_kind=BottomCenter shared_visual_size_px=208.088x457.319 shared_anchor_positions=bottom_center:1557.104,818.074|bottom_right:1679.014,818.074 previous_png_path=cache/old.png next_png_path=cache/new.png previous_zip_path=assets/old.zip next_zip_path=assets/new.zip previous_psd_path_in_zip=old/body.psd next_psd_path_in_zip=new/body.psd previous_config_scale=0.239 next_config_scale=0.145 restored_window_position=1557.104,818.074"
    );
}

#[test]
fn refresh_window_layout_log_message_reports_geometry_inputs() {
    let message = refresh_window_layout_message_for_test(
        "hot_reload",
        PlacementMode::SharedVisualSize,
        Some(VisualSizePx {
            width: 208.08792,
            height: 457.3187,
        }),
        Some(PlacementAnchorPositions {
            bottom_center: [1557.1039, 818.07404],
            bottom_right: [1679.0143, 818.07404],
        }),
        Path::new("cache/new.png"),
        Path::new("assets/new.zip"),
        Path::new("new/body.psd"),
        RefreshWindowLayoutDiagnosticsForTest {
            selected_anchor_kind: PlacementAnchorKind::BottomCenter,
            previous_layout: MascotWindowLayout::full(Vec2::new(100.0, 200.0)),
            next_layout: MascotWindowLayout::full(Vec2::new(120.0, 240.0)),
            viewport_info: Some(ViewportInfo {
                inner_origin: Pos2::new(400.0, 300.0),
                inner_to_outer_offset: Vec2::new(8.0, 30.0),
            }),
            preserved_anchor_position: Some(Pos2::new(450.0, 500.0)),
            next_inner_origin: Some(Pos2::new(390.0, 260.0)),
            next_outer_position: Some(Pos2::new(382.0, 230.0)),
        },
    );

    assert_eq!(
        message,
        "trigger=hot_reload action=refresh_window_layout window再配置を計算しました: placement_mode=SharedVisualSize selected_anchor_kind=BottomCenter shared_visual_size_px=208.088x457.319 shared_anchor_positions=bottom_center:1557.104,818.074|bottom_right:1679.014,818.074 previous_window_size=100.000,200.000 next_window_size=120.000,240.000 previous_anchor_offset=50.000,200.000 next_anchor_offset=60.000,240.000 previous_inner_origin=400.000,300.000 previous_inner_to_outer_offset=8.000,30.000 preserved_anchor_position=450.000,500.000 next_inner_origin=390.000,260.000 next_outer_position=382.000,230.000 configured_png_path=cache/new.png configured_zip_path=assets/new.zip configured_psd_path_in_zip=new/body.psd"
    );
}
