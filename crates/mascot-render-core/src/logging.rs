use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::workspace_paths::workspace_log_root;

pub(crate) struct PsdFailureLog<'a> {
    pub(crate) psd_path: &'a Path,
    pub(crate) metadata: &'a str,
    pub(crate) details: &'a [String],
    pub(crate) data_len: usize,
    pub(crate) backtrace: Option<&'a str>,
}

pub(crate) fn write_psd_failure_log(log: &PsdFailureLog<'_>) -> Option<PathBuf> {
    write_psd_failure_log_impl(log).ok()
}

pub(crate) fn clear_psd_failure_log(psd_path: &Path) {
    let log_path = psd_failure_log_path(psd_path);
    if log_path.exists() {
        let _ = fs::remove_file(log_path);
    }
}

pub(crate) fn psd_failure_log_path(psd_path: &Path) -> PathBuf {
    workspace_log_root().join(log_file_name(psd_path))
}

pub fn log_file_name(psd_path: &Path) -> String {
    let base_name = psd_path
        .file_name()
        .unwrap_or(psd_path.as_os_str())
        .to_string_lossy();
    let sanitized = base_name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string();

    format!(
        "psd-{}.log",
        if sanitized.is_empty() {
            "unknown"
        } else {
            &sanitized
        }
    )
}

fn write_psd_failure_log_impl(log: &PsdFailureLog<'_>) -> Result<PathBuf> {
    let log_dir = workspace_log_root();
    fs::create_dir_all(&log_dir).context("failed to create log directory")?;

    let log_path = psd_failure_log_path(log.psd_path);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();

    let mut body = String::new();
    body.push_str(&format!("timestamp_unix={timestamp}\n"));
    body.push_str(&format!("psd_path={}\n", log.psd_path.to_string_lossy()));
    body.push_str(&format!("psd_size_bytes={}\n", log.data_len));
    body.push_str(&format!("metadata={}\n", log.metadata));
    body.push_str("details:\n");
    for detail in log.details {
        body.push_str("- ");
        body.push_str(detail);
        body.push('\n');
    }

    if let Some(backtrace) = log.backtrace {
        body.push_str("\nbacktrace:\n");
        body.push_str(backtrace);
        if !backtrace.ends_with('\n') {
            body.push('\n');
        }
    }

    fs::write(&log_path, body)
        .with_context(|| format!("failed to write {}", log_path.to_string_lossy()))?;

    Ok(log_path)
}
