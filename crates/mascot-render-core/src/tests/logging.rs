use std::path::Path;

use crate::logging::log_file_name;

#[test]
fn log_file_name_is_stable_and_readable() {
    assert_eq!(log_file_name(Path::new("demo.psd")), "psd-demo.psd.log");
}
