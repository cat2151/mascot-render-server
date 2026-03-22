use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use eframe::egui::{self, Pos2, Vec2};
use mascot_render_core::{workspace_cache_root, MascotConfig};
use serde::{Deserialize, Serialize};

const WINDOW_HISTORY_VERSION: u32 = 1;
pub(crate) const WINDOW_HISTORY_SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
const WINDOW_HISTORY_NAME_STEM_LIMIT: usize = 64;

#[derive(Clone, Copy)]
pub(crate) struct ViewportInfo {
    pub(crate) inner_origin: Pos2,
    pub(crate) inner_to_outer_offset: Vec2,
    pub(crate) outer_origin: Pos2,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowHistoryFile {
    version: u32,
    outer_position: [f32; 2],
    updated_at: u64,
}

#[derive(Debug)]
pub(crate) struct WindowHistoryTracker {
    path: PathBuf,
    last_saved_position: Option<Pos2>,
    last_observed_position: Option<Pos2>,
    pending_position: Option<Pos2>,
    pending_changed_at: Option<Instant>,
}

impl WindowHistoryTracker {
    pub(crate) fn new(path: PathBuf, last_saved_position: Option<Pos2>) -> Self {
        Self {
            path,
            last_saved_position,
            last_observed_position: last_saved_position,
            pending_position: None,
            pending_changed_at: None,
        }
    }

    pub(crate) fn observe(&mut self, position: Pos2, now: Instant) -> Result<()> {
        let changed = match self.last_observed_position {
            Some(previous) => !same_position(previous, position),
            None => true,
        };
        self.last_observed_position = Some(position);
        if changed {
            self.pending_position = Some(position);
            self.pending_changed_at = Some(now);
        }

        if self.pending_changed_at.is_some_and(|changed_at| {
            now.duration_since(changed_at) >= WINDOW_HISTORY_SAVE_DEBOUNCE
        }) {
            self.flush()?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> Result<()> {
        let Some(position) = self.pending_position else {
            return Ok(());
        };
        if self
            .last_saved_position
            .is_some_and(|saved| same_position(saved, position))
        {
            self.pending_position = None;
            self.pending_changed_at = None;
            return Ok(());
        }

        save_window_position(&self.path, position)?;
        self.last_saved_position = Some(position);
        self.pending_position = None;
        self.pending_changed_at = None;
        Ok(())
    }
}

pub(crate) fn window_history_path(config: &MascotConfig) -> PathBuf {
    workspace_cache_root().join(format!(
        "history_server_{}.json",
        sanitize_window_history_name(&config.zip_path, &config.psd_path_in_zip)
    ))
}

pub(crate) fn load_window_position(path: &Path) -> Result<Option<Pos2>> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read window history {}", path.display()))?;
    let file: WindowHistoryFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse window history {}", path.display()))?;
    if file.version != WINDOW_HISTORY_VERSION {
        return Ok(None);
    }

    Ok(Some(sanitize_position(file.outer_position)?))
}

pub(crate) fn current_viewport_info(ctx: &egui::Context) -> Option<ViewportInfo> {
    ctx.input(|input| {
        let inner_rect = input.viewport().inner_rect?;
        let outer_origin = input
            .viewport()
            .outer_rect
            .map(|outer_rect| outer_rect.min)
            .unwrap_or(inner_rect.min);
        Some(ViewportInfo {
            inner_origin: inner_rect.min,
            inner_to_outer_offset: inner_rect.min - outer_origin,
            outer_origin,
        })
    })
}

fn save_window_position(path: &Path, position: Pos2) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let file = WindowHistoryFile {
        version: WINDOW_HISTORY_VERSION,
        outer_position: [position.x, position.y],
        updated_at: unix_timestamp(),
    };
    let json = serde_json::to_string_pretty(&file).context("failed to serialize window history")?;
    fs::write(path, json)
        .with_context(|| format!("failed to write window history {}", path.display()))
}

fn sanitize_position([x, y]: [f32; 2]) -> Result<Pos2> {
    if !x.is_finite() || !y.is_finite() {
        bail!("window history contains a non-finite position: [{x}, {y}]");
    }
    Ok(Pos2::new(x, y))
}

fn same_position(left: Pos2, right: Pos2) -> bool {
    (left.x - right.x).abs() < 0.5 && (left.y - right.y).abs() < 0.5
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

fn sanitize_window_history_name(zip_path: &Path, psd_path_in_zip: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    zip_path.hash(&mut hasher);
    psd_path_in_zip.hash(&mut hasher);
    let psd_name = psd_path_in_zip
        .file_stem()
        .unwrap_or(psd_path_in_zip.as_os_str())
        .to_string_lossy();
    let sanitized = psd_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    let capped = sanitized
        .chars()
        .take(WINDOW_HISTORY_NAME_STEM_LIMIT)
        .collect::<String>();

    if capped.is_empty() {
        format!("psd_{:016x}", hasher.finish())
    } else {
        format!("{capped}_{:016x}", hasher.finish())
    }
}
