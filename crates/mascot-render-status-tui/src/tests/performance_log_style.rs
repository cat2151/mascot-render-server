use ratatui::style::{Color, Modifier};

use crate::performance_log_style::styled_performance_log_line;

#[test]
fn styled_performance_log_line_highlights_slow_values() {
    let line = styled_performance_log_line(
        "latest 00:00:01 20959ms change_character completed q=4ms apply=20954ms settle=1ms top=refresh_mouth_flap_skins:12971ms char=demo",
    );

    let spans = line.spans;
    let latest = spans
        .iter()
        .find(|span| span.content.as_ref() == "latest")
        .expect("latest marker should exist");
    assert_eq!(latest.style.fg, Some(Color::Blue));

    let timestamp = spans
        .iter()
        .find(|span| span.content.as_ref() == "00:00:01")
        .expect("timestamp span should exist");
    assert_eq!(timestamp.style.fg, Some(Color::Gray));

    let total = spans
        .iter()
        .find(|span| span.content.as_ref() == "20959ms")
        .expect("total elapsed span should exist");
    assert_eq!(total.style.fg, Some(Color::Red));
    assert!(total.style.add_modifier.contains(Modifier::BOLD));

    let apply = spans
        .iter()
        .find(|span| span.content.as_ref() == "20954ms")
        .expect("apply span should exist");
    assert_eq!(apply.style.fg, Some(Color::Red));

    let top = spans
        .iter()
        .find(|span| span.content.as_ref() == "refresh_mouth_flap_skins:12971ms")
        .expect("top stage span should exist");
    assert_eq!(top.style.fg, Some(Color::Red));

    let completed = spans
        .iter()
        .find(|span| span.content.as_ref() == "completed")
        .expect("completed span should exist");
    assert_eq!(completed.style.fg, Some(Color::Green));
}

#[test]
fn styled_performance_log_line_highlights_failed_result() {
    let line = styled_performance_log_line(
        "900ms change_character failed q=1ms apply=899ms settle=0ms settled=false",
    );

    let spans = line.spans;
    let failed = spans
        .iter()
        .find(|span| span.content.as_ref() == "failed")
        .expect("failed span should exist");
    assert_eq!(failed.style.fg, Some(Color::Red));

    let settled = spans
        .iter()
        .find(|span| span.content.as_ref() == "false")
        .expect("settled=false value should exist");
    assert_eq!(settled.style.fg, Some(Color::Red));
}
