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
fn footer_help_line_mentions_esc_when_help_overlay_is_visible() {
    let mut app = App::loading(None);
    app.toggle_favorites_view();
    app.toggle_help_overlay();

    assert!(app.is_help_overlay_visible());
    assert!(
        line_text(app.help_line()).contains("Esc: close help"),
        "footer should describe Esc as closing help while help overlay is visible"
    );
    assert!(
        !line_text(app.help_line()).contains("Esc: close favorites"),
        "footer should prioritize the currently active Esc action"
    );
}

#[test]
fn log_overlay_closes_help_and_updates_footer_label() {
    let mut app = App::loading(None);
    app.toggle_help_overlay();

    app.show_log_overlay("mouth flap diagnostic");

    assert!(app.is_log_overlay_visible());
    assert!(!app.is_help_overlay_visible());
    assert!(line_text(app.help_line()).contains("Enter/Esc: close overlay"));

    app.clear_log_overlay();
    assert!(!app.is_log_overlay_visible());
    assert!(!line_text(app.help_line()).contains("Enter/Esc:"));
}

#[test]
fn footer_help_line_shows_esc_only_when_something_can_close() {
    let mut app = App::loading(None);

    assert!(
        !line_text(app.help_line()).contains("Esc:"),
        "footer should hide Esc when neither favorites nor log overlay is visible"
    );

    app.toggle_favorites_view();
    assert!(line_text(app.help_line()).contains("Esc: close favorites"));

    app.toggle_favorites_view();
    app.show_log_overlay("mouth flap diagnostic");
    assert!(line_text(app.help_line()).contains("Enter/Esc: close overlay"));
}

#[test]
fn help_overlay_includes_all_shortcuts() {
    let app = App::loading(None);
    let lines = app
        .help_overlay_lines()
        .into_iter()
        .map(line_text)
        .collect::<Vec<_>>();

    assert_eq!(lines[0], "Press ? or Esc to close help.");
    assert!(
        lines.contains(&"Space/Enter: toggle selected layer".to_string()),
        "help overlay should describe layer toggling"
    );
    assert!(
        lines.contains(&"f: save current PSD to favorites (ZIP / PSD or layer pane)".to_string()),
        "help overlay should describe favorite saving"
    );
    assert!(
        lines.contains(
            &"v: open/close favorites list, Esc: close favorites list, Enter/Esc: close overlay"
                .to_string()
        ),
        "help overlay should describe favorites list and log overlay closing"
    );
    assert!(
        lines.contains(&"e: toggle favorite ensemble true/false".to_string()),
        "help overlay should describe favorite ensemble toggle"
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
        line_text(app.help_line())
            .contains("f: favorite | v: favorites | e: ensemble | -/+: mascot scale"),
        "footer should describe favorites, ensemble, and scale shortcuts"
    );
}

fn line_text(line: Line<'static>) -> String {
    line.spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect()
}
