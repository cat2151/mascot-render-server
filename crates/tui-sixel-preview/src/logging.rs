use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use ratatui::layout::Rect;
use ratatui_image::picker::{Capability, Picker, ProtocolType};
use serde::Serialize;

use crate::workspace_paths::workspace_log_root;

const SIXEL_PREVIEW_TIMING_LOG_VERSION: u32 = 1;
const TERMINAL_PROTOCOL_LOG_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct TerminalProtocolLog {
    version: u32,
    timestamp_unix: u64,
    source: String,
    protocol_type: String,
    font_size: TerminalFontSize,
    capabilities: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TerminalFontSize {
    width: u16,
    height: u16,
}

#[derive(Debug, Serialize)]
struct SixelPreviewTimingLog {
    version: u32,
    timestamp_unix: u64,
    png_path: PathBuf,
    image_pixels: ImagePixels,
    area_cells: AreaCells,
    png_load_ms: f64,
    sixel_encode_ms: f64,
    sixel_data_len: usize,
}

#[derive(Debug, Serialize)]
struct ImagePixels {
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize)]
struct AreaCells {
    width: u16,
    height: u16,
}

pub(crate) fn append_sixel_preview_timing_log(
    png_path: &Path,
    image_pixels: (u32, u32),
    area: Rect,
    png_load_ms: f64,
    sixel_encode_ms: f64,
    sixel_data_len: usize,
) -> Option<PathBuf> {
    append_sixel_preview_timing_log_impl(
        png_path,
        image_pixels,
        area,
        png_load_ms,
        sixel_encode_ms,
        sixel_data_len,
    )
    .ok()
}

pub(crate) fn write_terminal_protocol_log(
    picker: &Picker,
    source: &str,
    error: Option<&str>,
) -> Option<PathBuf> {
    write_terminal_protocol_log_impl(picker, source, error).ok()
}

pub(crate) fn terminal_protocol_log_path() -> PathBuf {
    workspace_log_root().join("terminal-protocol.json")
}

pub(crate) fn sixel_preview_timing_log_path() -> PathBuf {
    workspace_log_root().join("sixel-preview-timings.jsonl")
}

pub(crate) fn protocol_type_name(protocol_type: ProtocolType) -> &'static str {
    match protocol_type {
        ProtocolType::Halfblocks => "halfblocks",
        ProtocolType::Sixel => "sixel",
        ProtocolType::Kitty => "kitty",
        ProtocolType::Iterm2 => "iterm2",
    }
}

pub(crate) fn capability_name(capability: &Capability) -> String {
    match capability {
        Capability::Kitty => "kitty".to_string(),
        Capability::Sixel => "sixel".to_string(),
        Capability::RectangularOps => "rectangular_ops".to_string(),
        Capability::CellSize(Some((width, height))) => {
            format!("cell_size:{width}x{height}")
        }
        Capability::CellSize(None) => "cell_size:unknown".to_string(),
        Capability::TextSizingProtocol => "text_sizing_protocol".to_string(),
    }
}

fn write_terminal_protocol_log_impl(
    picker: &Picker,
    source: &str,
    error: Option<&str>,
) -> Result<PathBuf> {
    let log_path = terminal_protocol_log_path();
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).context("failed to create log directory")?;
    }

    let (font_width, font_height) = picker.font_size();
    let log = TerminalProtocolLog {
        version: TERMINAL_PROTOCOL_LOG_VERSION,
        timestamp_unix: unix_timestamp(),
        source: source.to_string(),
        protocol_type: protocol_type_name(picker.protocol_type()).to_string(),
        font_size: TerminalFontSize {
            width: font_width,
            height: font_height,
        },
        capabilities: picker.capabilities().iter().map(capability_name).collect(),
        error: error.map(ToOwned::to_owned),
    };

    let json =
        serde_json::to_string_pretty(&log).context("failed to serialize terminal protocol log")?;
    fs::write(&log_path, json)
        .with_context(|| format!("failed to write {}", log_path.to_string_lossy()))?;

    Ok(log_path)
}

fn append_sixel_preview_timing_log_impl(
    png_path: &Path,
    image_pixels: (u32, u32),
    area: Rect,
    png_load_ms: f64,
    sixel_encode_ms: f64,
    sixel_data_len: usize,
) -> Result<PathBuf> {
    let log_path = sixel_preview_timing_log_path();
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).context("failed to create log directory")?;
    }

    let log = SixelPreviewTimingLog {
        version: SIXEL_PREVIEW_TIMING_LOG_VERSION,
        timestamp_unix: unix_timestamp(),
        png_path: png_path.to_path_buf(),
        image_pixels: ImagePixels {
            width: image_pixels.0,
            height: image_pixels.1,
        },
        area_cells: AreaCells {
            width: area.width,
            height: area.height,
        },
        png_load_ms: round_ms(png_load_ms),
        sixel_encode_ms: round_ms(sixel_encode_ms),
        sixel_data_len,
    };

    let json =
        serde_json::to_string(&log).context("failed to serialize sixel preview timing log")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open {}", log_path.to_string_lossy()))?;
    writeln!(file, "{json}")
        .with_context(|| format!("failed to write {}", log_path.to_string_lossy()))?;

    Ok(log_path)
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}
