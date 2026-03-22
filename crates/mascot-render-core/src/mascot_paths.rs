use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::cache::default_cache_root;
use crate::workspace_paths::{local_data_path, relative_to_known_root};

const MASCOT_CONFIG_PATH: &str = "mascot-render-server.toml";

pub fn mascot_config_path() -> PathBuf {
    local_data_path(MASCOT_CONFIG_PATH)
}

pub fn mascot_runtime_state_path(config_path: &Path) -> PathBuf {
    let state_stem = config_path
        .file_stem()
        .filter(|value| !value.is_empty())
        .or_else(|| config_path.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "mascot-render-server".to_string());
    let config_hash = hash_config_path(config_path);
    default_cache_root().join(format!("{state_stem}-{config_hash}.state.json"))
}

pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

fn hash_config_path(config_path: &Path) -> String {
    let mut hasher = Sha256::new();
    let relative_path = relative_to_known_root(config_path);
    hasher.update(relative_path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
