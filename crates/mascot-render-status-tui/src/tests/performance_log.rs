use crate::performance_log::{
    compact_performance_log_line, compact_performance_log_lines, tail_lines,
};

#[test]
fn tail_lines_keeps_recent_performance_records() {
    let lines = tail_lines("one\ntwo\nthree\n", 2);

    assert_eq!(lines, vec!["two".to_string(), "three".to_string()]);
}

#[test]
fn tail_lines_reports_empty_log() {
    let lines = tail_lines("", 6);

    assert_eq!(lines, vec!["empty".to_string()]);
}

#[test]
fn compact_performance_log_line_prioritizes_elapsed_time() {
    let line = "[2026-04-12 00:00:00.000Z] INFO event=post_to_status_settled action=change_character result=completed elapsed_ms=42 queue_ms=3 apply_ms=34 settle_ms=5 requested_at_unix_ms=100 applying_at_unix_ms=103 applied_at_unix_ms=137 completed_at_unix_ms=142 status_settled=true texture_changed=true stage_ms=resolve_character_skin:4ms,refresh_closed_eye_skin:12ms,refresh_mouth_flap_skins:18ms previous_displayed_png_path=N:\\cache\\old.png displayed_png_path=N:\\cache\\very\\deep\\dir\\very-long-rendered-file-name-for-display.png configured_png_path=N:\\cache\\very\\deep\\dir\\very-long-rendered-file-name-for-display.png command_summary=character=demo";

    let compact = compact_performance_log_line(line);

    assert!(
        compact.starts_with("00:00:00 42ms change_character completed"),
        "elapsed time should lead the compact performance line: {compact}"
    );
    assert!(
        compact.contains("q=3ms")
            && compact.contains("apply=34ms")
            && compact.contains("settle=5ms"),
        "coarse segments should remain visible: {compact}"
    );
    assert!(
        compact.contains("top=refresh_mouth_flap_skins:18ms"),
        "slowest internal stage should remain visible: {compact}"
    );
    assert!(
        compact.contains("char=demo"),
        "target character should be visible without relying on long png paths: {compact}"
    );
    assert!(
        !compact.contains("requested_at_unix_ms") && !compact.contains("N:\\cache\\very\\deep"),
        "compact line should avoid full path details: {compact}"
    );
}

#[test]
fn compact_performance_log_lines_shows_latest_first_with_labels() {
    let old_line = "[2026-04-12 00:00:00.000Z] INFO event=post_to_status_settled action=change_character result=completed elapsed_ms=4981 queue_ms=3 apply_ms=4977 settle_ms=1 status_settled=true texture_changed=true stage_ms=refresh_closed_eye_skin:1000ms displayed_png_path=N:\\cache\\old.png command_summary=character=old";
    let latest_line = "[2026-04-12 00:00:01.000Z] INFO event=post_to_status_settled action=change_character result=completed elapsed_ms=20959 queue_ms=4 apply_ms=20954 settle_ms=1 status_settled=true texture_changed=true stage_ms=refresh_mouth_flap_skins:12971ms displayed_png_path=N:\\cache\\latest.png command_summary=character=latest";

    let compact =
        compact_performance_log_lines(vec![old_line.to_string(), latest_line.to_string()]);

    assert_eq!(compact.len(), 2);
    assert!(
        compact[0].starts_with("latest 00:00:01 20959ms"),
        "latest event should be first: {compact:?}"
    );
    assert!(
        compact[1].starts_with("prev1 00:00:00 4981ms"),
        "previous event should be clearly labeled: {compact:?}"
    );
}
