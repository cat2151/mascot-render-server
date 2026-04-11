use mascot_render_protocol::ServerWorkStatus;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::state::StatusTuiState;
use crate::ui::{draw, help_overlay_area, post_result_panel_height, work_status_text};

#[test]
fn help_overlay_area_is_centered_and_sized_to_content() {
    let area = Rect::new(0, 0, 100, 40);

    let overlay = help_overlay_area(area);

    assert!(overlay.width < area.width);
    assert!(overlay.height < area.height);
    assert_eq!(overlay.x, (area.width - overlay.width) / 2);
    assert_eq!(overlay.y, (area.height - overlay.height) / 2);
}

#[test]
fn help_overlay_area_is_clamped_for_small_terminals() {
    let area = Rect::new(5, 7, 20, 8);

    let overlay = help_overlay_area(area);

    assert!(overlay.width <= area.width);
    assert!(overlay.height <= area.height);
    assert!(overlay.x >= area.x);
    assert!(overlay.y >= area.y);
}

#[test]
fn work_status_text_includes_elapsed_age() {
    let work = ServerWorkStatus {
        kind: "reload_config_if_needed".to_string(),
        stage: "load_active_skin".to_string(),
        summary: "png_path=skin.png".to_string(),
        started_at_unix_ms: 1_000,
        updated_at_unix_ms: 1_250,
    };

    let text = work_status_text(Some(&work), 2_500);

    assert!(text.contains("kind: reload_config_if_needed"));
    assert!(text.contains("stage: load_active_skin"));
    assert!(text.contains("summary: png_path=skin.png"));
    assert!(text.contains("elapsed: 1.5s"));
}

#[test]
fn draw_renders_test_post_result_panel_with_visible_result_prefix() {
    let mut state = StatusTuiState::new();
    state.record_test_post_success(
        "change-character random cached PSD: generated_character_name=demo random_zip=cache/demo.zip random_psd=demo/body.psd random_png=cache/demo/body.png"
            .to_string(),
        1_234,
    );
    let backend = TestBackend::new(120, 26);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");

    terminal
        .draw(|frame| draw(frame, &state))
        .expect("status TUI should render");

    let rendered = format!("{}", terminal.backend());
    assert!(
        rendered.contains("Test POST Result"),
        "POST result panel should be titled in rendered TUI:\n{rendered}"
    );
    assert!(
        rendered.contains("ok (1.2s):"),
        "POST result prefix should be visible before long details:\n{rendered}"
    );
}

#[test]
fn post_result_panel_height_reserves_about_one_quarter_of_the_screen() {
    assert_eq!(post_result_panel_height(40), 10);
    assert_eq!(post_result_panel_height(24), 6);
    assert_eq!(post_result_panel_height(4), 4);
}
