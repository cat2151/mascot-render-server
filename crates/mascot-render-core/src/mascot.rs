use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use image::ImageReader;

use crate::mascot_motion::{
    AlwaysBendConfig, BounceAnimationConfig, HeadHitbox, IdleSinkAnimationConfig,
    SquashBounceAnimationConfig,
};
pub use crate::mascot_paths::{
    mascot_config_path, mascot_runtime_state_path, psd_viewer_tui_activity_path, unix_timestamp,
};
#[path = "mascot/config_files.rs"]
mod config_files;

use config_files::{MascotRuntimeStateFile, MascotStaticConfigFile};

const DEFAULT_MAX_EDGE: f32 = 480.0;
const DEFAULT_SCREEN_HEIGHT_RATIO: f32 = 0.33;
const MASCOT_RUNTIME_STATE_VERSION: u32 = 1;
const PSD_VIEWER_TUI_ACTIVITY_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MascotImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MascotConfig {
    pub png_path: PathBuf,
    pub scale: Option<f32>,
    pub favorite_ensemble_scale: Option<f32>,
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub display_diff_path: Option<PathBuf>,
    pub always_idle_sink_enabled: bool,
    pub always_bend: AlwaysBendConfig,
    pub favorite_ensemble_enabled: bool,
    pub transparent_background_click_through: bool,
    pub flash_blue_background_on_transparent_input: bool,
    pub head_hitbox: HeadHitbox,
    pub bounce: BounceAnimationConfig,
    pub squash_bounce: SquashBounceAnimationConfig,
    pub always_idle_sink: IdleSinkAnimationConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MascotTarget {
    pub png_path: PathBuf,
    pub scale: Option<f32>,
    pub favorite_ensemble_scale: Option<f32>,
    pub zip_path: PathBuf,
    pub psd_path_in_zip: PathBuf,
    pub display_diff_path: Option<PathBuf>,
}

pub fn parse_mascot_config_path(args: impl IntoIterator<Item = OsString>) -> Result<PathBuf> {
    let mut args = args.into_iter();
    let _program = args.next();
    let mut config_path = None;

    while let Some(arg) = args.next() {
        if arg == "--config" {
            let Some(value) = args.next() else {
                bail!("--config requires a file path");
            };
            config_path = Some(PathBuf::from(value));
            continue;
        }

        if arg.to_string_lossy().starts_with('-') {
            bail!("unsupported argument '{}'", arg.to_string_lossy());
        }

        bail!(
            "unsupported positional argument '{}'; use --config <path> or the default config at {}",
            arg.to_string_lossy(),
            mascot_config_path().display()
        );
    }

    Ok(config_path.unwrap_or_else(mascot_config_path))
}

pub fn load_mascot_config(config_path: &Path) -> Result<MascotConfig> {
    let static_config = load_mascot_static_config_file(config_path)?;
    let runtime_state_path = mascot_runtime_state_path(config_path);
    let runtime_target = load_mascot_runtime_target(config_path)?;
    validate_mascot_target(&runtime_target, &runtime_state_path)?;
    let favorite_ensemble_enabled =
        effective_favorite_ensemble_enabled(config_path, static_config.favorite_ensemble_enabled)?;

    Ok(MascotConfig {
        png_path: runtime_target.png_path,
        scale: runtime_target.scale,
        favorite_ensemble_scale: runtime_target.favorite_ensemble_scale,
        zip_path: runtime_target.zip_path,
        psd_path_in_zip: runtime_target.psd_path_in_zip,
        display_diff_path: runtime_target.display_diff_path,
        always_idle_sink_enabled: static_config.always_idle_sink_enabled,
        always_bend: static_config.always_bend,
        favorite_ensemble_enabled,
        transparent_background_click_through: static_config.transparent_background_click_through,
        flash_blue_background_on_transparent_input: static_config
            .flash_blue_background_on_transparent_input,
        head_hitbox: static_config.head_hitbox,
        bounce: static_config.bounce,
        squash_bounce: static_config.squash_bounce,
        always_idle_sink: static_config.always_idle_sink,
    })
}

fn effective_favorite_ensemble_enabled(
    config_path: &Path,
    favorite_ensemble_enabled: bool,
) -> Result<bool> {
    if !favorite_ensemble_enabled {
        return Ok(false);
    }

    Ok(!psd_viewer_tui_is_active(config_path)?)
}

fn psd_viewer_tui_is_active(config_path: &Path) -> Result<bool> {
    let activity_path = psd_viewer_tui_activity_path(config_path);
    let bytes = match fs::read_to_string(&activity_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            eprintln!(
                "warning: ignoring unreadable psd-viewer-tui activity heartbeat {}: {error:#}",
                activity_path.display()
            );
            return Ok(false);
        }
    };
    let heartbeat = bytes.trim();
    if heartbeat.is_empty() {
        eprintln!(
            "warning: ignoring empty psd-viewer-tui activity heartbeat {}",
            activity_path.display()
        );
        return Ok(false);
    }

    let active_at = match heartbeat.parse::<u64>() {
        Ok(active_at) => active_at,
        Err(error) => {
            eprintln!(
                "warning: ignoring invalid psd-viewer-tui activity heartbeat {}: {:?} ({error})",
                activity_path.display(),
                heartbeat
            );
            return Ok(false);
        }
    };
    let now = unix_timestamp();
    if active_at > now {
        eprintln!(
            "warning: ignoring future psd-viewer-tui activity heartbeat {}: active_at={} now={}",
            activity_path.display(),
            active_at,
            now
        );
        return Ok(false);
    }

    Ok(now.saturating_sub(active_at) <= PSD_VIEWER_TUI_ACTIVITY_TTL.as_secs())
}

pub fn write_mascot_config(config_path: &Path, target: &MascotTarget) -> Result<()> {
    normalize_mascot_static_config(config_path)?;

    let state_path = mascot_runtime_state_path(config_path);
    validate_mascot_target(target, &state_path)?;
    if let Some(parent) = state_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&MascotRuntimeStateFile::from(target))
        .context("failed to serialize mascot runtime state")?;
    fs::write(&state_path, json)
        .with_context(|| format!("failed to write {}", state_path.display()))?;
    Ok(())
}

fn load_mascot_static_config_file(config_path: &Path) -> Result<MascotStaticConfigFile> {
    if !config_path.exists() {
        return Ok(MascotStaticConfigFile::default());
    }

    let bytes = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    toml::from_str::<MascotStaticConfigFile>(&bytes)
        .with_context(|| format!("failed to parse {}", config_path.display()))
}

fn load_mascot_runtime_target(config_path: &Path) -> Result<MascotTarget> {
    let state_path = mascot_runtime_state_path(config_path);
    if state_path.exists() {
        return load_mascot_runtime_state_file(&state_path)
            .map(MascotRuntimeStateFile::into_target);
    }

    bail!(
        "mascot runtime state '{}' is missing; select a PSD in psd-viewer-tui first",
        state_path.display()
    );
}

fn load_mascot_runtime_state_file(state_path: &Path) -> Result<MascotRuntimeStateFile> {
    let bytes =
        fs::read(state_path).with_context(|| format!("failed to read {}", state_path.display()))?;
    let state = serde_json::from_slice::<MascotRuntimeStateFile>(&bytes)
        .with_context(|| format!("failed to parse {}", state_path.display()))?;
    validate_version(
        state.version,
        MASCOT_RUNTIME_STATE_VERSION,
        state_path,
        "mascot runtime state",
    )?;
    Ok(state)
}

fn validate_version(version: u32, supported_version: u32, path: &Path, label: &str) -> Result<()> {
    if version != supported_version {
        bail!(
            "unsupported {label} version {} in '{}'",
            version,
            path.display()
        );
    }
    Ok(())
}

fn normalize_mascot_static_config(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        return write_mascot_static_config_file(config_path, &MascotStaticConfigFile::default());
    }

    let bytes = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    toml::from_str::<MascotStaticConfigFile>(&bytes)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    Ok(())
}

fn write_mascot_static_config_file(
    config_path: &Path,
    config: &MascotStaticConfigFile,
) -> Result<()> {
    if let Some(parent) = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let toml =
        toml::to_string_pretty(config).context("failed to serialize mascot static config")?;
    fs::write(config_path, toml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(())
}

fn validate_mascot_target(target: &MascotTarget, state_path: &Path) -> Result<()> {
    if target.png_path.as_os_str().is_empty() {
        bail!(
            "mascot runtime state '{}' must point to a .png file, got an empty path",
            state_path.display()
        );
    }
    if target.png_path.extension().and_then(|value| value.to_str()) != Some("png") {
        bail!(
            "mascot runtime state '{}' must point to a .png file, got '{}'",
            state_path.display(),
            target.png_path.display()
        );
    }
    validate_scale(target.scale, state_path)?;
    validate_scale(target.favorite_ensemble_scale, state_path)?;

    if target.zip_path.as_os_str().is_empty() {
        bail!(
            "mascot runtime state '{}' must include zip_path; select a PSD in psd-viewer-tui first",
            state_path.display()
        );
    }
    if target.psd_path_in_zip.as_os_str().is_empty() {
        bail!(
            "mascot runtime state '{}' must include psd_path_in_zip; select a PSD in psd-viewer-tui first",
            state_path.display()
        );
    }
    Ok(())
}

pub fn load_mascot_image(png_path: &Path) -> Result<MascotImageData> {
    let image = ImageReader::open(png_path)
        .with_context(|| format!("failed to open {}", png_path.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", png_path.display()))?
        .into_rgba8();

    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(anyhow!("image '{}' has zero size", png_path.display()));
    }

    Ok(MascotImageData {
        path: png_path.to_path_buf(),
        width,
        height,
        rgba: image.into_raw(),
    })
}

pub fn mascot_window_size(width: u32, height: u32, scale: Option<f32>) -> [f32; 2] {
    let scale = mascot_scale(width, height, scale);
    [width as f32 * scale, height as f32 * scale]
}

pub fn default_mascot_scale_for_screen_height(image_height: u32, screen_height_px: u16) -> f32 {
    if image_height == 0 || screen_height_px == 0 {
        return 1.0;
    }

    (screen_height_px as f32 * DEFAULT_SCREEN_HEIGHT_RATIO) / image_height as f32
}

fn mascot_scale(width: u32, height: u32, configured_scale: Option<f32>) -> f32 {
    configured_scale
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or_else(|| legacy_fit_scale(width, height, DEFAULT_MAX_EDGE))
}

fn legacy_fit_scale(width: u32, height: u32, max_edge: f32) -> f32 {
    let width = width as f32;
    let height = height as f32;
    let largest_edge = width.max(height);
    if largest_edge <= max_edge {
        return 1.0;
    }

    max_edge / largest_edge
}

fn validate_scale(scale: Option<f32>, config_path: &Path) -> Result<()> {
    if scale.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        bail!(
            "mascot config '{}' must have a positive finite scale, got {:?}",
            config_path.display(),
            scale
        );
    }
    Ok(())
}
