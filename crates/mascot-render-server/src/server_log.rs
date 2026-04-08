use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::{Context, Result};
use mascot_render_core::local_data_root;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

const SERVER_LOG_PATH: &str = "logs/server.log";
const POST_REQUEST_LOG_PATH: &str = "logs/post-request.log";
static LOG_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn init_server_log() -> Result<PathBuf> {
    let path = server_log_path();
    ensure_server_log_exists(&path)?;
    Ok(path)
}

pub fn log_server_info(message: impl AsRef<str>) {
    log_server("INFO", message.as_ref(), false);
}

pub fn log_server_error(message: impl AsRef<str>) {
    let message = message.as_ref();
    eprintln!("{message}");
    log_server("ERROR", message, true);
}

pub fn log_post_request(message: impl AsRef<str>) {
    log_post_request_with_level("post request log", "INFO", message.as_ref(), false);
}

pub fn log_post_request_error(message: impl AsRef<str>) {
    let message = message.as_ref();
    log_post_request_with_level("post request log", "ERROR", message, false);
}

fn log_post_request_with_level(
    log_kind: &str,
    level: &str,
    message: &str,
    already_printed_to_stderr: bool,
) {
    let path = post_request_log_path();
    if let Err(error) = append_log_record(log_kind, &path, level, message) {
        if !already_printed_to_stderr {
            eprintln!("{message}");
        }
        eprintln!(
            "failed to append post request log {}: {error:#}",
            path.display()
        );
    }
}

fn log_server(level: &str, message: &str, already_printed_to_stderr: bool) {
    let path = server_log_path();
    if let Err(error) = append_log_record("server log", &path, level, message) {
        if !already_printed_to_stderr {
            eprintln!("{message}");
        }
        eprintln!("failed to append server log {}: {error:#}", path.display());
    }
}

fn server_log_path() -> PathBuf {
    local_data_root().join(SERVER_LOG_PATH)
}

fn post_request_log_path() -> PathBuf {
    local_data_root().join(POST_REQUEST_LOG_PATH)
}

fn ensure_server_log_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).with_context(|| {
            format!("failed to create server log directory {}", parent.display())
        })?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open server log {}", path.display()))?;
    Ok(())
}

fn append_log_record(log_kind: &str, path: &Path, level: &str, message: &str) -> Result<()> {
    let _guard = log_write_lock()
        .lock()
        .unwrap_or_else(|_| panic!("{log_kind} write lock should not be poisoned"));
    if let Some(parent) = path.parent() {
        create_dir_all(parent).with_context(|| {
            format!("failed to create {log_kind} directory {}", parent.display())
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {log_kind} {}", path.display()))?;
    file.write_all(format_log_record(level, message).as_bytes())
        .with_context(|| format!("failed to write {log_kind} {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush {log_kind} {}", path.display()))?;
    Ok(())
}

fn log_write_lock() -> &'static Mutex<()> {
    LOG_WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

fn format_log_record(level: &str, message: &str) -> String {
    format_log_record_at(level, message, SystemTime::now())
}

fn format_log_record_at(level: &str, message: &str, now: SystemTime) -> String {
    let timestamp = human_readable_timestamp(now);
    let mut output = String::new();

    if message.is_empty() {
        output.push_str(&format!("[{timestamp}] {level} \n"));
        return output;
    }

    for line in message.lines() {
        output.push_str(&format!("[{timestamp}] {level} {line}\n"));
    }

    output
}

fn human_readable_timestamp(now: SystemTime) -> String {
    OffsetDateTime::from(now)
        .to_offset(UtcOffset::UTC)
        .format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]Z"
        ))
        .unwrap_or_else(|_| "1970-01-01 00:00:00.000Z".to_string())
}

#[cfg(test)]
pub(crate) fn append_log_record_for_test(path: &Path, level: &str, message: &str) -> Result<()> {
    append_log_record("test log", path, level, message)
}

#[cfg(test)]
pub(crate) fn format_log_record_for_test(level: &str, message: &str, now: SystemTime) -> String {
    format_log_record_at(level, message, now)
}

#[cfg(test)]
pub(crate) fn post_request_log_path_for_test() -> PathBuf {
    post_request_log_path()
}

#[cfg(test)]
pub(crate) fn server_log_path_for_test() -> PathBuf {
    server_log_path()
}
