use std::path::{Path, PathBuf};

use anyhow::Result;
use mascot_render_server::{log_server_error, log_server_info, log_server_skin_info};

use super::MascotApp;

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

impl MascotApp {
    pub(super) fn log_rendered_skin_if_changed(&mut self, png_path: &Path) {
        if !should_log_rendered_skin(self.last_logged_skin_path.as_deref(), png_path) {
            return;
        }
        self.last_logged_skin_path = Some(png_path.to_path_buf());
        log_server_skin_info(rendered_skin_message(png_path));
    }

    pub(super) fn clear_last_logged_skin_path(&mut self) {
        clear_rendered_skin_path(&mut self.last_logged_skin_path);
    }
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

#[cfg(test)]
pub(crate) use change_skin_failure_message as change_skin_failure_message_for_test;
#[cfg(test)]
pub(crate) use change_skin_stage_message as change_skin_stage_message_for_test;
#[cfg(test)]
pub(crate) use change_skin_success_message as change_skin_success_message_for_test;
#[cfg(test)]
pub(crate) use clear_rendered_skin_path as clear_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use record_rendered_skin_path as record_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use rendered_skin_message as rendered_skin_message_for_test;
#[cfg(test)]
pub(crate) use should_log_rendered_skin as should_log_rendered_skin_for_test;
