use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_DATA_ROOT_ENV: &str = "MASCOT_RENDER_SERVER_DATA_ROOT";
const TEST_ROOT_DIR_NAME: &str = "mascot-render-server-tests";

pub fn init_process_test_data_root() {
    let root = process_test_data_root();
    std::fs::create_dir_all(root).unwrap_or_else(|error| {
        panic!(
            "failed to create test data root {}: {error}",
            root.display()
        )
    });
    std::env::set_var(TEST_DATA_ROOT_ENV, root);
}

#[macro_export]
macro_rules! install_test_data_root {
    () => {
        #[::ctor::ctor]
        fn __init_process_test_data_root() {
            ::mascot_render_test_support::init_process_test_data_root();
        }
    };
}

fn process_test_data_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(build_process_test_data_root)
}

fn build_process_test_data_root() -> PathBuf {
    let pid = std::process::id();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir()
        .join(TEST_ROOT_DIR_NAME)
        .join(format!("{}-{pid}-{started_at}", test_binary_name()))
}

fn test_binary_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|value| sanitize_path_component(&value.to_string_lossy()))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown-test-binary".to_string())
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}
