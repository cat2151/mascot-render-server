use std::path::Path;

use anyhow::Result;
use mascot_render_server::{log_server_error, log_server_info};

pub(crate) fn change_skin_stage_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    format!(
        "trigger=control_command action=change_skin skin変更を処理中です: stage={stage} from={} to={}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn change_skin_success_message(
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    format!(
        "trigger=control_command action=change_skin skin変更に成功しました: from={} to={} runtime_state_path={} persisted_png_path={}",
        previous_png_path.display(),
        png_path.display(),
        runtime_state_path.display(),
        persisted_png_path.display()
    )
}

pub(crate) fn change_skin_failure_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    format!(
        "trigger=control_command action=change_skin skin変更に失敗しました: stage={stage} from={} to={} error={error_detail}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn run_change_skin_stage<T>(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    log_server_info(change_skin_stage_message(
        previous_png_path,
        png_path,
        stage,
    ));
    operation().map_err(|error| {
        log_server_error(change_skin_failure_message(
            previous_png_path,
            png_path,
            stage,
            &format!("{error:#}"),
        ));
        error
    })
}

#[cfg(test)]
pub(crate) use change_skin_failure_message as change_skin_failure_message_for_test;
#[cfg(test)]
pub(crate) use change_skin_stage_message as change_skin_stage_message_for_test;
#[cfg(test)]
pub(crate) use change_skin_success_message as change_skin_success_message_for_test;
