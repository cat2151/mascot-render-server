use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const TUI_HISTORY_VERSION: u32 = 1;
const TUI_HISTORY_FILE_NAME: &str = "history_tui.json";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TuiHistory {
    pub(crate) selected_node: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TuiHistoryFile {
    version: u32,
    selected_node: Option<usize>,
    updated_at: u64,
}

pub(crate) fn load_tui_history(cache_root: &Path) -> Result<TuiHistory> {
    let path = tui_history_path(cache_root);
    if !path.exists() {
        return Ok(TuiHistory::default());
    }

    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read TUI history {}", path.display()))?;
    match serde_json::from_slice::<TuiHistoryFile>(&bytes) {
        Ok(file) if file.version == TUI_HISTORY_VERSION => Ok(TuiHistory {
            selected_node: file.selected_node,
        }),
        Ok(_) => Ok(TuiHistory::default()),
        Err(_) => Ok(TuiHistory::default()),
    }
}

pub(crate) fn save_tui_history(cache_root: &Path, history: &TuiHistory) -> Result<()> {
    fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create cache root {}", cache_root.display()))?;

    let file = TuiHistoryFile {
        version: TUI_HISTORY_VERSION,
        selected_node: history.selected_node,
        updated_at: unix_timestamp(),
    };

    let path = tui_history_path(cache_root);
    let json = serde_json::to_string_pretty(&file).context("failed to serialize TUI history")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write TUI history {}", path.display()))?;
    Ok(())
}

fn tui_history_path(cache_root: &Path) -> PathBuf {
    cache_root.join(TUI_HISTORY_FILE_NAME)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}
