use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mascot_render_core::{
    default_eye_blink_targets, local_data_root, migrate_eye_blink_layers, workspace_cache_root,
    EyeBlinkTarget,
};
use serde::{Deserialize, Serialize};

const TUI_CONFIG_PATH: &str = "psd-viewer-tui.toml";
const TUI_CONFIG_VERSION: u32 = 1;
const TUI_RUNTIME_STATE_VERSION: u32 = 1;
pub(crate) const DEFAULT_LAYER_SCROLL_MARGIN_RATIO: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TuiConfig {
    pub(crate) layer_scroll_margin_ratio: f32,
    pub(crate) eye_blink_targets: Vec<EyeBlinkTarget>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
            eye_blink_targets: default_eye_blink_targets(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TuiRuntimeState {
    pub(crate) legacy_mascot_scale: Option<f32>,
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
#[serde(default)]
struct TuiConfigFile {
    version: u32,
    layer_scroll_margin_ratio: f32,
    #[serde(default = "default_eye_blink_targets")]
    eye_blink_targets: Vec<EyeBlinkTarget>,
}

impl Default for TuiConfigFile {
    fn default() -> Self {
        Self {
            version: TUI_CONFIG_VERSION,
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
            eye_blink_targets: default_eye_blink_targets(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct TuiRuntimeStateFile {
    version: u32,
    #[serde(default, alias = "mascot_scale")]
    legacy_mascot_scale: Option<f32>,
    #[serde(default)]
    psd_states: Vec<PsdRuntimeState>,
    updated_at: u64,
}

impl Default for TuiRuntimeStateFile {
    fn default() -> Self {
        Self {
            version: TUI_RUNTIME_STATE_VERSION,
            legacy_mascot_scale: None,
            psd_states: Vec::new(),
            updated_at: 0,
        }
    }
}

impl From<&TuiRuntimeState> for TuiRuntimeStateFile {
    fn from(state: &TuiRuntimeState) -> Self {
        Self {
            version: TUI_RUNTIME_STATE_VERSION,
            legacy_mascot_scale: sanitize_scale(state.legacy_mascot_scale),
            psd_states: sanitize_psd_states(state.psd_states.clone()),
            updated_at: unix_timestamp(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct LegacyTuiConfigFile {
    version: u32,
    mascot_scale: Option<f32>,
    layer_scroll_margin_ratio: f32,
    #[serde(default = "default_eye_blink_targets")]
    eye_blink_targets: Vec<EyeBlinkTarget>,
    updated_at: u64,
}

impl Default for LegacyTuiConfigFile {
    fn default() -> Self {
        Self {
            version: TUI_CONFIG_VERSION,
            mascot_scale: None,
            layer_scroll_margin_ratio: DEFAULT_LAYER_SCROLL_MARGIN_RATIO,
            eye_blink_targets: default_eye_blink_targets(),
            updated_at: 0,
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
            eye_blink_targets: sanitize_eye_blink_targets(file.eye_blink_targets),
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

    if !config_path.exists() {
        return Ok(TuiRuntimeState::default());
    }

    let bytes = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read TUI config {}", config_path.display()))?;
    match toml::from_str::<LegacyTuiConfigFile>(&bytes) {
        Ok(file) if file.version == TUI_CONFIG_VERSION => Ok(TuiRuntimeState {
            legacy_mascot_scale: sanitize_scale(file.mascot_scale),
            psd_states: Vec::new(),
        }),
        Ok(_) => Ok(TuiRuntimeState::default()),
        Err(_) => Ok(TuiRuntimeState::default()),
    }
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
        eye_blink_targets: sanitize_eye_blink_targets(config.eye_blink_targets.clone()),
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

pub(crate) fn ensure_tui_config_split(
    config_path: &Path,
    config: &TuiConfig,
    runtime_state: &TuiRuntimeState,
) -> Result<()> {
    if !config_path.exists() || static_config_needs_normalization(config_path)? {
        save_tui_config(config_path, config)?;
    }

    let runtime_state_path = tui_runtime_state_path(config_path);
    if !runtime_state_path.exists()
        && (runtime_state.legacy_mascot_scale.is_some() || !runtime_state.psd_states.is_empty())
    {
        save_tui_runtime_state(config_path, runtime_state)?;
    }

    Ok(())
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
    match serde_json::from_slice::<TuiRuntimeStateFile>(&bytes) {
        Ok(file) if file.version == TUI_RUNTIME_STATE_VERSION => Ok(TuiRuntimeState {
            legacy_mascot_scale: sanitize_scale(file.legacy_mascot_scale),
            psd_states: sanitize_psd_states(file.psd_states),
        }),
        Ok(_) => Ok(TuiRuntimeState::default()),
        Err(_) => Ok(TuiRuntimeState::default()),
    }
}

fn static_config_needs_normalization(path: &Path) -> Result<bool> {
    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read TUI config {}", path.display()))?;
    Ok(bytes.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("mascot_scale ")
            || trimmed.starts_with("mascot_scale=")
            || trimmed.starts_with("updated_at ")
            || trimmed.starts_with("updated_at=")
    }))
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

fn sanitize_eye_blink_targets(targets: Vec<EyeBlinkTarget>) -> Vec<EyeBlinkTarget> {
    targets
        .into_iter()
        .filter_map(sanitize_eye_blink_target)
        .collect()
}

fn sanitize_eye_blink_target(target: EyeBlinkTarget) -> Option<EyeBlinkTarget> {
    let psd_file_name = sanitize_psd_file_name(&target.psd_file_name);
    let mut first_layer_name = target.first_layer_name.trim().to_string();
    let mut second_layer_name = target.second_layer_name.trim().to_string();
    if let Some((migrated_first, migrated_second)) =
        migrate_eye_blink_layers(&psd_file_name, &first_layer_name, &second_layer_name)
    {
        first_layer_name = migrated_first.to_string();
        second_layer_name = migrated_second.to_string();
    }
    if psd_file_name.is_empty() || first_layer_name.is_empty() || second_layer_name.is_empty() {
        return None;
    }

    Some(EyeBlinkTarget {
        psd_file_name,
        first_layer_name,
        second_layer_name,
    })
}

fn sanitize_psd_file_name(psd_file_name: &str) -> String {
    let trimmed = psd_file_name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    Path::new(trimmed)
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| trimmed.to_string())
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
