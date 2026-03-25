use ratatui::text::Line;

use crate::app::App;

#[test]
fn footer_help_line_lists_quit_before_help() {
    let app = App::loading(None);

    assert!(
        line_text(app.help_line()).starts_with("q: quit | ?: help |"),
        "footer should list quit before help"
    );
}

#[test]
fn question_mark_toggles_help_overlay_visibility() {
    let mut app = App::loading(None);

    assert!(!app.is_help_overlay_visible());

    app.toggle_help_overlay();
    assert!(app.is_help_overlay_visible());
    assert!(line_text(app.help_line()).contains("?: close help"));

    app.toggle_help_overlay();
    assert!(!app.is_help_overlay_visible());
}

#[test]
fn help_overlay_includes_all_shortcuts() {
    let app = App::loading(None);
    let lines = app
        .help_overlay_lines()
        .into_iter()
        .map(line_text)
        .collect::<Vec<_>>();

    assert_eq!(lines[0], "Press ? to close help.");
    assert!(
        lines.contains(&"Space/Enter: toggle selected layer".to_string()),
        "help overlay should describe layer toggling"
    );
    assert!(
        lines.contains(&"f: save current PSD to favorites (layer pane)".to_string()),
        "help overlay should describe favorite saving"
    );
    assert!(
        lines.contains(&"v/Esc: close favorites list (v also opens it)".to_string()),
        "help overlay should describe favorites list toggle"
    );
}

#[test]
fn footer_help_line_mentions_space_and_enter_for_toggle() {
    let app = App::loading(None);

    assert!(
        line_text(app.help_line()).contains("Space/Enter: toggle"),
        "footer should describe both Space and Enter as toggle keys"
    );
    assert!(
        line_text(app.help_line()).contains("f: favorite | v: favorites"),
        "footer should describe favorites shortcuts"
    );
}

fn line_text(line: Line<'static>) -> String {
    line.spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect()
}
