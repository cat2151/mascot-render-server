use std::path::Path;

use crate::mascot_app::{
    change_skin_failure_message_for_test, change_skin_stage_message_for_test,
    change_skin_success_message_for_test,
};

#[test]
fn change_skin_stage_log_message_includes_stage_and_paths() {
    let message = change_skin_stage_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        "load_skin",
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_skin skin変更を処理中です: stage=load_skin from=cache/anko/normal.png to=cache/zunda/normal.png"
    );
}

#[test]
fn change_skin_success_log_message_reports_success() {
    let message = change_skin_success_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        Path::new("config/mascot-render-server.runtime.json"),
        Path::new("cache/zunda/normal.png"),
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_skin skin変更に成功しました: from=cache/anko/normal.png to=cache/zunda/normal.png runtime_state_path=config/mascot-render-server.runtime.json persisted_png_path=cache/zunda/normal.png"
    );
}

#[test]
fn change_skin_failure_log_message_reports_stage_and_error() {
    let message = change_skin_failure_message_for_test(
        Path::new("cache/anko/normal.png"),
        Path::new("cache/zunda/normal.png"),
        "refresh_mouth_flap_skins",
        "failed to refresh mouth-flap skins",
    );

    assert_eq!(
        message,
        "trigger=control_command action=change_skin skin変更に失敗しました: stage=refresh_mouth_flap_skins from=cache/anko/normal.png to=cache/zunda/normal.png error=failed to refresh mouth-flap skins"
    );
}
