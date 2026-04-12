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
        compact.contains(
            "parts=refresh_mouth_flap_skins:18ms|refresh_closed_eye_skin:12ms|resolve_character_skin:4ms"
        ),
        "internal stage breakdown should remain visible: {compact}"
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
fn compact_performance_log_line_breaks_down_mouth_flap_leaf_stages() {
    let line = "[2026-04-12 00:00:00.000Z] INFO event=post_to_status_settled action=timeline_mouth_flap result=completed elapsed_ms=20000 queue_ms=1 apply_ms=2 settle_ms=19997 status_settled=true texture_changed=true stage_ms=refresh_pending_auxiliary_skins.refresh_mouth_flap_skins:20000ms,mouth_flap.inspect_psd:14000ms,mouth_flap.inspect_psd.zip_extract:6000ms,mouth_flap.inspect_psd.psd_entry_build:7000ms,mouth_flap.render_open_png:5000ms,mouth_flap.render_open_png.psd_analyze:4500ms,mouth_flap.render_closed_png.compose_save_png:3000ms displayed_png_path=N:\\cache\\latest.png command_summary=timeline=mouth_flap";

    let compact = compact_performance_log_line(line);

    assert!(
        compact.contains("top=mouth_flap.inspect_psd.psd_entry_build:7000ms"),
        "mouth-flap top should point at a leaf stage, not only the parent total: {compact}"
    );
    assert!(
        compact.contains(
            "parts=mouth_flap.inspect_psd.psd_entry_build:7000ms|mouth_flap.inspect_psd.zip_extract:6000ms|mouth_flap.render_open_png.psd_analyze:4500ms"
        ),
        "mouth-flap breakdown should show the slow leaf stages: {compact}"
    );
}

#[test]
fn compact_performance_log_line_breaks_down_skin_load() {
    let line = "[2026-04-12 00:00:00.000Z] INFO event=skin_load stage=cache_miss_loaded elapsed_ms=984 cache_lookup_ms=0 raw_rgba_cache_hit=true raw_rgba_cache_status=hit raw_rgba_meta_read_ms=1 raw_rgba_read_ms=9 read_file_ms=0 decode_png_ms=0 detail_cache_hit=true detail_cache_read_ms=7 alpha_mask_ms=0 content_bounds_ms=0 detail_cache_write_ms=0 texture_alloc_ms=118 cache_insert_ms=1 file_bytes=123456 rgba_bytes=4194304 image_size=1024x1024 evicted_count=1 png_path=N:\\cache\\very\\long\\default.png";

    let compact = compact_performance_log_line(line);

    assert!(
        compact.contains(
            "skin file_load raw=hit raw_meta=1ms raw_read=9ms read=0ms decode=0ms detail_cache=true detail_read=7ms alpha=0ms bounds=0ms detail_write=0ms texture=118ms"
        ),
        "skin load breakdown should stay visible: {compact}"
    );
    assert!(compact.contains("png=default.png"));
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
