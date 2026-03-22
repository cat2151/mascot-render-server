use std::path::PathBuf;

use encoding_rs::SHIFT_JIS;

use crate::archive::{decode_zip_path, safe_entry_path};

#[test]
fn decode_zip_path_falls_back_to_shift_jis() {
    let (encoded, _, had_errors) = SHIFT_JIS.encode("ずんだもん/readme.txt");

    assert!(!had_errors);
    assert_eq!(decode_zip_path(&encoded), "ずんだもん/readme.txt");
}

#[test]
fn safe_entry_path_normalizes_backslashes() {
    let (encoded, _, had_errors) = SHIFT_JIS.encode("ずんだもん\\basic.psd");

    assert!(!had_errors);
    assert_eq!(
        safe_entry_path(&encoded),
        Some(PathBuf::from("ずんだもん").join("basic.psd"))
    );
}

#[test]
fn safe_entry_path_rejects_parent_traversal() {
    assert_eq!(safe_entry_path(br"..\evil.psd"), None);
}
