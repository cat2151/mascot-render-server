use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const WORKSPACE_STATE_VERSION: u32 = 2;

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkspaceState {
    pub(crate) selected_zip_cache_key: Option<String>,
    pub(crate) selected_psd_path: Option<PathBuf>,
    pub(crate) selected_node: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct WorkspaceStateFile {
    version: u32,
    selected_zip_cache_key: Option<String>,
    selected_psd_path: Option<PathBuf>,
    #[serde(default, skip_serializing)]
    selected_node: Option<usize>,
    updated_at: u64,
}

pub(crate) fn load_workspace_state(cache_root: &Path) -> Result<WorkspaceState> {
    let path = workspace_state_path(cache_root);
    if !path.exists() {
        return Ok(WorkspaceState::default());
    }

    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read workspace state {}", path.display()))?;
    match serde_json::from_slice::<WorkspaceStateFile>(&bytes) {
        Ok(file) if file.version == WORKSPACE_STATE_VERSION => Ok(WorkspaceState {
            selected_zip_cache_key: file.selected_zip_cache_key,
            selected_psd_path: file.selected_psd_path,
            selected_node: file.selected_node,
        }),
        Ok(_) => Ok(WorkspaceState::default()),
        Err(_) => Ok(WorkspaceState::default()),
    }
}

pub(crate) fn save_workspace_state(
    cache_root: &Path,
    selected_zip_cache_key: Option<&str>,
    selected_psd_path: Option<&Path>,
) -> Result<()> {
    fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create cache root {}", cache_root.display()))?;

    let file = WorkspaceStateFile {
        version: WORKSPACE_STATE_VERSION,
        selected_zip_cache_key: selected_zip_cache_key.map(ToOwned::to_owned),
        selected_psd_path: selected_psd_path.map(Path::to_path_buf),
        selected_node: None,
        updated_at: unix_timestamp(),
    };

    let path = workspace_state_path(cache_root);
    let json =
        serde_json::to_string_pretty(&file).context("failed to serialize workspace state")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write workspace state {}", path.display()))?;
    Ok(())
}

fn workspace_state_path(cache_root: &Path) -> PathBuf {
    cache_root.join("index.json")
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}
