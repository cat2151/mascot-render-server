use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use mascot_render_core::{local_data_root, workspace_cache_root, MouthFlapTarget};
use serde::{Deserialize, Serialize};

const TUI_CONFIG_PATH: &str = "psd-viewer-tui.toml";
const TUI_CONFIG_VERSION: u32 = 1;
const TUI_RUNTIME_STATE_VERSION: u32 = 1;
pub(crate) const DEFAULT_LAYER_SCROLL_MARGIN_RATIO: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TuiConfig {
    pub(crate) layer_scroll_margin_ratio: f32,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TuiRuntimeState {
    pub(crate) psd_states: Vec<PsdRuntimeState>,
}

impl TuiRuntimeState {
    pub(crate) fn mascot_scale_for_psd(
        &self,
        zip_path: &Path,
        psd_path_in_zip: &Path,
    ) -> Option<f32> {
        self.psd_states
            .iter()
            .find(|state| state.zip_path == zip_path && state.psd_path_in_zip == psd_path_in_zip)
            .and_then(|state| state.mascot_scale)
    }

    pub(crate) fn set_mascot_scale_for_psd(
        &mut self,
        zip_path: PathBuf,
        psd_path_in_zip: PathBuf,
        mascot_scale: Option<f32>,
    ) {
        let mascot_scale = sanitize_scale(mascot_scale);
        if let Some(state) = self
            .psd_states
            .iter_mut()
            .find(|state| state.zip_path == zip_path && state.psd_path_in_zip == psd_path_in_zip)
        {
            state.mascot_scale = mascot_scale;
        } else if mascot_scale.is_some() {
            self.psd_states.push(PsdRuntimeState {
                zip_path,
                psd_path_in_zip,
                mascot_scale,
            });
        }
        self.psd_states = sanitize_psd_states(std::mem::take(&mut self.psd_states));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PsdRuntimeState {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) mascot_scale: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct TuiConfigFile {
    version: u32,
    layer_scroll_margin_ratio: f32,
    #[serde(default, skip_serializing, rename = "eye_blink_targets")]
    _eye_blink_targets: Vec<LegacyEyeBlinkTarget>,
    #[serde(default, skip_serializing, rename = "mouth_flap_targets")]
    _mouth_flap_targets: Vec<MouthFlapTarget>,
}

impl Default for TuiConfigFile {
    fn default() -> Self {
        Self {
            version: TUI_CONFIG_VERSION,
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
            _eye_blink_targets: Vec::new(),
            _mouth_flap_targets: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
/// Legacy read-only shape for deprecated `[[eye_blink_targets]]` entries.
///
/// These entries are accepted only so older `psd-viewer-tui.toml` files can
/// still load without dropping unrelated settings. They are ignored by the
/// application and never written back out.
struct LegacyEyeBlinkTarget {
    psd_file_name: String,
    first_layer_name: String,
    second_layer_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct TuiRuntimeStateFile {
    version: u32,
    #[serde(default)]
    psd_states: Vec<PsdRuntimeState>,
    updated_at: u64,
}

impl Default for TuiRuntimeStateFile {
    fn default() -> Self {
        Self {
            version: TUI_RUNTIME_STATE_VERSION,
            psd_states: Vec::new(),
            updated_at: 0,
        }
    }
}

impl From<&TuiRuntimeState> for TuiRuntimeStateFile {
    fn from(state: &TuiRuntimeState) -> Self {
        Self {
            version: TUI_RUNTIME_STATE_VERSION,
            psd_states: sanitize_psd_states(state.psd_states.clone()),
            updated_at: unix_timestamp(),
        }
    }
}

pub(crate) fn load_tui_config(path: &Path) -> Result<TuiConfig> {
    if !path.exists() {
        return Ok(TuiConfig::default());
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read TUI config {}", path.display()))?;
    match toml::from_str::<TuiConfigFile>(&bytes) {
        Ok(file) if file.version == TUI_CONFIG_VERSION => Ok(TuiConfig {
            layer_scroll_margin_ratio: sanitize_layer_scroll_margin_ratio(
                file.layer_scroll_margin_ratio,
            ),
        }),
        Ok(_) => Ok(TuiConfig::default()),
        Err(_) => Ok(TuiConfig::default()),
    }
}

pub(crate) fn load_tui_runtime_state(config_path: &Path) -> Result<TuiRuntimeState> {
    let state_path = tui_runtime_state_path(config_path);
    if state_path.exists() {
        return load_tui_runtime_state_file(&state_path);
    }

    Ok(TuiRuntimeState::default())
}

pub(crate) fn save_tui_config(path: &Path, config: &TuiConfig) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let file = TuiConfigFile {
        version: TUI_CONFIG_VERSION,
        layer_scroll_margin_ratio: sanitize_layer_scroll_margin_ratio(
            config.layer_scroll_margin_ratio,
        ),
        _eye_blink_targets: Vec::new(),
        _mouth_flap_targets: Vec::new(),
    };
    let toml = toml::to_string_pretty(&file).context("failed to serialize TUI config")?;
    fs::write(path, toml).with_context(|| format!("failed to write TUI config {}", path.display()))
}

pub(crate) fn save_tui_runtime_state(config_path: &Path, state: &TuiRuntimeState) -> Result<()> {
    let path = tui_runtime_state_path(config_path);
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&TuiRuntimeStateFile::from(state))
        .context("failed to serialize TUI runtime state")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write TUI runtime state {}", path.display()))
}

pub(crate) fn tui_config_path() -> PathBuf {
    local_data_root().join(TUI_CONFIG_PATH)
}

pub(crate) fn tui_runtime_state_path(config_path: &Path) -> PathBuf {
    workspace_cache_root().join(format!(
        "{}.state.json",
        sanitize_runtime_state_name(config_path)
    ))
}

fn load_tui_runtime_state_file(path: &Path) -> Result<TuiRuntimeState> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read TUI runtime state {}", path.display()))?;
    let file = serde_json::from_slice::<TuiRuntimeStateFile>(&bytes)
        .with_context(|| format!("failed to parse TUI runtime state {}", path.display()))?;
    if file.version != TUI_RUNTIME_STATE_VERSION {
        bail!(
            "unsupported TUI runtime state version {} in '{}'",
            file.version,
            path.display()
        );
    }
    Ok(TuiRuntimeState {
        psd_states: sanitize_psd_states(file.psd_states),
    })
}

fn sanitize_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn sanitize_psd_states(states: Vec<PsdRuntimeState>) -> Vec<PsdRuntimeState> {
    states
        .into_iter()
        .filter_map(|state| {
            let zip_path = state.zip_path;
            let psd_path_in_zip = state.psd_path_in_zip;
            let mascot_scale = sanitize_scale(state.mascot_scale);
            if !has_meaningful_path(&zip_path) || !has_meaningful_path(&psd_path_in_zip) {
                return None;
            }
            mascot_scale.map(|mascot_scale| PsdRuntimeState {
                zip_path,
                psd_path_in_zip,
                mascot_scale: Some(mascot_scale),
            })
        })
        .collect()
}

fn sanitize_layer_scroll_margin_ratio(ratio: f32) -> f32 {
    if !ratio.is_finite() {
        return DEFAULT_LAYER_SCROLL_MARGIN_RATIO;
    }
    ratio.clamp(0.0, 0.49)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

fn has_meaningful_path(path: &Path) -> bool {
    path.components().next().is_some() && !path.to_string_lossy().trim().is_empty()
}

fn sanitize_runtime_state_name(path: &Path) -> String {
    let relative_path = path
        .strip_prefix(local_data_root())
        .or_else(|_| path.strip_prefix(mascot_render_core::workspace_root()))
        .unwrap_or(path);
    let sanitized = relative_path
        .to_string_lossy()
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "psd_viewer_tui".to_string()
    } else {
        sanitized
    }
}
