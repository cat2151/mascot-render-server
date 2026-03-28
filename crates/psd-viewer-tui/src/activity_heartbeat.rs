use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mascot_render_core::psd_viewer_tui_activity_path;

const HEARTBEAT_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) struct ActivityHeartbeat {
    path: PathBuf,
    last_refreshed_at: Instant,
}

impl ActivityHeartbeat {
    pub(crate) fn start(config_path: &Path, now: Instant) -> Result<Self> {
        Self::start_with_path(psd_viewer_tui_activity_path(config_path), now)
    }

    fn start_with_path(path: PathBuf, now: Instant) -> Result<Self> {
        let mut heartbeat = Self {
            path,
            last_refreshed_at: now.checked_sub(HEARTBEAT_REFRESH_INTERVAL).unwrap_or(now),
        };
        heartbeat.refresh_if_due(now)?;
        Ok(heartbeat)
    }

    pub(crate) fn refresh_if_due(&mut self, now: Instant) -> Result<()> {
        if now.duration_since(self.last_refreshed_at) < HEARTBEAT_REFRESH_INTERVAL {
            return Ok(());
        }

        if let Some(parent) = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        fs::write(&self.path, unix_timestamp().to_string())
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        self.last_refreshed_at = now;
        Ok(())
    }
}

impl Drop for ActivityHeartbeat {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path) {
            if error.kind() != ErrorKind::NotFound {
                eprintln!(
                    "warning: failed to remove psd-viewer-tui activity heartbeat {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
impl ActivityHeartbeat {
    pub(crate) fn start_with_path_for_test(path: PathBuf, now: Instant) -> Result<Self> {
        Self::start_with_path(path, now)
    }
}
