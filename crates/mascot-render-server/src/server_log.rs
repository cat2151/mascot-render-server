use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mascot_render_core::local_data_root;

const SERVER_LOG_PATH: &str = "logs/server.log";
const POST_REQUEST_LOG_PATH: &str = "logs/post-request.log";
static SERVER_LOG_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
    log_post_request_with_level("INFO", message.as_ref());
}

pub fn log_post_request_error(message: impl AsRef<str>) {
    let message = message.as_ref();
    eprintln!("{message}");
    log_post_request_with_level("ERROR", message);
}

fn log_post_request_with_level(level: &str, message: &str) {
    let path = post_request_log_path();
    if let Err(error) = append_log_record(&path, level, message) {
        eprintln!("{message}");
        eprintln!(
            "failed to append post request log {}: {error:#}",
            path.display()
        );
    }
}

fn log_server(level: &str, message: &str, already_printed_to_stderr: bool) {
    let path = server_log_path();
    if let Err(error) = append_log_record(&path, level, message) {
        if !already_printed_to_stderr {
            eprintln!("{message}");
        }
        eprintln!("failed to append server log {}: {error:#}", path.display());
    }
}

fn server_log_path() -> PathBuf {
    std::env::current_dir()
        .map(|path| path.join(SERVER_LOG_PATH))
        .unwrap_or_else(|_| PathBuf::from(SERVER_LOG_PATH))
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

fn append_log_record(path: &Path, level: &str, message: &str) -> Result<()> {
    let _guard = server_log_write_lock()
        .lock()
        .expect("server log write lock should not be poisoned");
    if let Some(parent) = path.parent() {
        create_dir_all(parent).with_context(|| {
            format!("failed to create server log directory {}", parent.display())
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open server log {}", path.display()))?;
    file.write_all(format_log_record(level, message).as_bytes())
        .with_context(|| format!("failed to write server log {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush server log {}", path.display()))?;
    Ok(())
}

fn server_log_write_lock() -> &'static Mutex<()> {
    SERVER_LOG_WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

fn format_log_record(level: &str, message: &str) -> String {
    let timestamp = unix_timestamp();
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

fn unix_timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:03}", duration.as_secs(), duration.subsec_millis()),
        Err(_) => "0.000".to_string(),
    }
}

#[cfg(test)]
pub(crate) fn append_log_record_for_test(path: &Path, level: &str, message: &str) -> Result<()> {
    append_log_record(path, level, message)
}

#[cfg(test)]
pub(crate) fn post_request_log_path_for_test() -> PathBuf {
    post_request_log_path()
}

#[cfg(test)]
pub(crate) fn server_log_path_for_test() -> PathBuf {
    server_log_path()
}
