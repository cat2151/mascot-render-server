use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::server_log::append_log_record_for_test;

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

    fs::remove_dir_all(
        log_path
            .parent()
            .and_then(|path| path.parent())
            .expect("log path should include nested directories"),
    )
    .expect("temporary server log directory should be removable");
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
        .join(format!("mascot-render-server-{prefix}-{unique_suffix}"))
        .join("logs")
        .join("server.log")
}
