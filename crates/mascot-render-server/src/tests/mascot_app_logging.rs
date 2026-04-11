use std::path::Path;
use std::path::PathBuf;

use crate::mascot_app::{
    change_character_failure_message_for_test, change_character_stage_message_for_test,
    change_character_success_message_for_test, clear_rendered_skin_path_for_test,
    record_rendered_skin_path_for_test, rendered_skin_message_for_test,
    should_log_rendered_skin_for_test,
};

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
