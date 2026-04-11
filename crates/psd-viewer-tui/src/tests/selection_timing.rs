use crate::app::format_timing_log_message_for_test;

#[test]
fn timing_log_message_keeps_action_summary_and_stage_durations() {
    let message = format_timing_log_message_for_test(
        "refresh_selected_psd_state",
        "zip=demo.zip\npsd=body.psd",
        123,
        &[("to_document", 10), ("sync_current_mascot_config", 90)],
    );

    assert!(message.contains("trigger=selection_timing"));
    assert!(message.contains("action=refresh_selected_psd_state"));
    assert!(message.contains("total_ms=123"));
    assert!(message.contains("summary=\"zip=demo.zip psd=body.psd\""));
    assert!(message.contains("stages=[to_document:10ms,sync_current_mascot_config:90ms]"));
}
