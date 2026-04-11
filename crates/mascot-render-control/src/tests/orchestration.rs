use std::fs;

use crate::orchestration::{startup_diagnostics_context_for_test, tail_chars_for_test};

#[test]
fn tail_chars_keeps_short_text() {
    assert_eq!(tail_chars_for_test("abc", 10), "abc");
}

#[test]
fn tail_chars_keeps_last_unicode_chars() {
    assert_eq!(tail_chars_for_test("aあbいc", 3), "bいc");
}

#[test]
fn startup_diagnostics_context_includes_log_tail() {
    let path = std::env::temp_dir().join(format!(
        "mascot-render-control-startup-diagnostics-{}.log",
        std::process::id()
    ));
    fs::write(&path, "command=server\n\nError: missing png\n").expect("should write diagnostics");

    let context = startup_diagnostics_context_for_test(&path);

    assert!(
        context.contains("startup diagnostics tail:"),
        "unexpected context: {context}"
    );
    assert!(
        context.contains("Error: missing png"),
        "unexpected context: {context}"
    );
    let _ = fs::remove_file(path);
}
