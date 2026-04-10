use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::logging::{
    append_log_record_for_test, control_log_path_for_test, format_log_record_for_test,
    server_log_path_for_test, server_skin_log_path_for_test, startup_diagnostics_dir_for_test,
};
use crate::paths::control_local_data_root_for_test;

#[test]
fn append_log_record_creates_parent_directory_and_writes_message() {
    let log_path = unique_test_log_path("append-log-record");
    append_log_record_for_test(&log_path, "INFO", "skin変更しました")
        .expect("server log write should succeed");

    let log_contents =
        fs::read_to_string(&log_path).expect("server log should be readable after writing");

    assert!(
        log_contents.contains("INFO skin変更しました"),
        "unexpected log contents: {log_contents}"
    );

    remove_test_log_dir(&log_path);
}

#[test]
fn format_log_record_uses_human_readable_utc_timestamp() {
    let record = format_log_record_for_test(
        "INFO",
        "skin変更に成功しました",
        UNIX_EPOCH + Duration::from_millis(0),
    );

    assert_eq!(
        record,
        "[1970-01-01 00:00:00.000Z] INFO skin変更に成功しました\n"
    );
}

#[test]
fn append_log_record_prefixes_each_multiline_line() {
    let log_path = unique_test_log_path("append-log-record-multiline");
    append_log_record_for_test(
        &log_path,
        "INFO",
        "request:\n{\n  \"png_path\": \"cache/demo/variation.png\"\n}",
    )
    .expect("multiline server log write should succeed");

    let log_contents =
        fs::read_to_string(&log_path).expect("server log should be readable after writing");
    let line_count = log_contents.lines().count();

    assert_eq!(line_count, 4, "unexpected log contents: {log_contents}");
    assert!(
        log_contents
            .lines()
            .all(|line| line.contains("INFO") && !line.trim().is_empty()),
        "unexpected log contents: {log_contents}"
    );

    remove_test_log_dir(&log_path);
}

#[test]
fn append_log_record_serializes_concurrent_writes() {
    let log_path = unique_test_log_path("append-log-record-concurrent");
    let start = Arc::new(Barrier::new(5));
    let mut handles = Vec::new();

    for index in 0..4 {
        let log_path = log_path.clone();
        let start = Arc::clone(&start);
        handles.push(thread::spawn(move || {
            start.wait();
            append_log_record_for_test(&log_path, "INFO", &format!("message-{index}"))
                .expect("concurrent server log write should succeed");
        }));
    }

    start.wait();
    for handle in handles {
        handle.join().expect("worker thread should complete");
    }

    let log_contents =
        fs::read_to_string(&log_path).expect("server log should be readable after writing");
    let mut lines = log_contents.lines().collect::<Vec<_>>();
    lines.sort_unstable();

    assert_eq!(lines.len(), 4, "unexpected log contents: {log_contents}");
    for index in 0..4 {
        assert!(
            lines
                .iter()
                .any(|line| line.ends_with(&format!("INFO message-{index}"))),
            "missing message-{index} in log contents: {log_contents}"
        );
    }

    remove_test_log_dir(&log_path);
}

#[test]
fn control_log_path_lives_under_local_data_root_logs_directory() {
    assert_eq!(
        control_log_path_for_test(),
        control_local_data_root_for_test()
            .join("logs")
            .join("control.log")
    );
}

#[test]
fn server_log_path_lives_under_local_data_root_logs_directory() {
    assert_eq!(
        server_log_path_for_test(),
        control_local_data_root_for_test()
            .join("logs")
            .join("server.log")
    );
}

#[test]
fn server_skin_log_path_lives_under_local_data_root_logs_directory() {
    assert_eq!(
        server_skin_log_path_for_test(),
        control_local_data_root_for_test()
            .join("logs")
            .join("server_skin.log")
    );
}

#[test]
fn startup_diagnostics_dir_lives_under_local_data_root_logs_directory() {
    assert_eq!(
        startup_diagnostics_dir_for_test(),
        control_local_data_root_for_test()
            .join("logs")
            .join("startup")
    );
}

fn remove_test_log_dir(log_path: &Path) {
    fs::remove_dir_all(
        log_path
            .parent()
            .and_then(|path| path.parent())
            .expect("log path should include nested directories"),
    )
    .expect("temporary server log directory should be removable");
}

fn unique_test_log_path(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("mascot-render-control-{prefix}-{unique_suffix}"))
        .join("logs")
        .join("server.log")
}
