use mascot_render_protocol::ServerWorkStatus;
use ratatui::layout::Rect;

use crate::ui::{help_overlay_area, work_status_text};

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
