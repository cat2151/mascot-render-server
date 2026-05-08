use std::path::{Path, PathBuf};

use anyhow::Result;
use mascot_render_control::{log_server_error, log_server_info};

pub(crate) fn change_character_stage_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    control_command_stage_message("change_character", previous_png_path, png_path, stage)
}

pub(crate) fn preview_target_stage_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    control_command_stage_message("preview_target", previous_png_path, png_path, stage)
}

fn control_command_stage_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
) -> String {
    format!(
        "trigger=control_command action={action} character変更を処理中です: stage={stage} from={} to={}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn change_character_success_message(
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    control_command_success_message(
        "change_character",
        previous_png_path,
        png_path,
        runtime_state_path,
        persisted_png_path,
    )
}

pub(crate) fn preview_target_success_message(
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    control_command_success_message(
        "preview_target",
        previous_png_path,
        png_path,
        runtime_state_path,
        persisted_png_path,
    )
}

fn control_command_success_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    runtime_state_path: &Path,
    persisted_png_path: &Path,
) -> String {
    format!(
        "trigger=control_command action={action} character変更に成功しました: from={} to={} runtime_state_path={} persisted_png_path={}",
        previous_png_path.display(),
        png_path.display(),
        runtime_state_path.display(),
        persisted_png_path.display()
    )
}

pub(crate) fn change_character_failure_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    control_command_failure_message(
        "change_character",
        previous_png_path,
        png_path,
        stage,
        error_detail,
    )
}

pub(crate) fn preview_target_failure_message(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    control_command_failure_message(
        "preview_target",
        previous_png_path,
        png_path,
        stage,
        error_detail,
    )
}

fn control_command_failure_message(
    action: &str,
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    error_detail: &str,
) -> String {
    format!(
        "trigger=control_command action={action} character変更に失敗しました: stage={stage} from={} to={} error={error_detail}",
        previous_png_path.display(),
        png_path.display()
    )
}

pub(crate) fn rendered_skin_message(png_path: &Path) -> String {
    let png_file_name = match png_path.file_name() {
        Some(file_name) => file_name.to_string_lossy().into_owned(),
        None => png_path.display().to_string(),
    };
    format!(
        "trigger=render action=display_skin displayed_png_path={} displayed_png_file_name={png_file_name}",
        png_path.display()
    )
}

pub(crate) fn run_change_character_stage<T>(
    previous_png_path: &Path,
    png_path: &Path,
    stage: &str,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    log_server_info(change_character_stage_message(
        previous_png_path,
        png_path,
        stage,
    ));
    operation().map_err(|error| {
        log_server_error(change_character_failure_message(
            previous_png_path,
            png_path,
            stage,
            &format!("{error:#}"),
        ));
        error
    })
}

pub(crate) fn should_log_rendered_skin(
    last_logged_skin_path: Option<&Path>,
    png_path: &Path,
) -> bool {
    last_logged_skin_path != Some(png_path)
}

#[cfg(test)]
pub(crate) fn record_rendered_skin_path(
    last_logged_skin_path: &mut Option<PathBuf>,
    png_path: &Path,
) -> bool {
    if !should_log_rendered_skin(last_logged_skin_path.as_deref(), png_path) {
        return false;
    }
    *last_logged_skin_path = Some(png_path.to_path_buf());
    true
}

pub(crate) fn clear_rendered_skin_path(last_logged_skin_path: &mut Option<PathBuf>) {
    *last_logged_skin_path = None;
}
