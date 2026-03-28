use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mascot_render_core::{
    local_data_root, psd_viewer_tui_activity_path, unix_timestamp, write_mascot_config, Core,
    DisplayDiff, LayerVisibilityOverride, MascotConfig, MascotTarget, RenderRequest,
};
use serde::{Deserialize, Serialize};

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "favorites.toml";

pub const FAVORITE_SHUFFLE_INTERVAL: Duration = Duration::from_secs(60);
const PSD_VIEWER_TUI_ACTIVITY_TTL: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct FavoriteShufflePlaylist {
    favorites_path: PathBuf,
    state: FavoriteShuffleState,
}

#[derive(Debug)]
struct FavoriteShuffleState {
    next_rotation_at: Instant,
    last_selected_key: Option<FavoriteKey>,
    known_favorites: Vec<FavoriteEntry>,
    remaining: VecDeque<FavoriteEntry>,
    shuffle_seed: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct FavoriteEntry {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) psd_file_name: String,
    pub(crate) mascot_scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FavoriteKey {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FavoriteEntryFile {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    #[serde(default)]
    psd_file_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    visibility_overrides: Vec<LayerVisibilityOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mascot_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_position: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    favorite_ensemble_position: Option<[f32; 2]>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteEntryFile>,
}

impl FavoriteEntry {
    pub(crate) fn key(&self) -> FavoriteKey {
        FavoriteKey {
            zip_path: self.zip_path.clone(),
            psd_path_in_zip: self.psd_path_in_zip.clone(),
        }
    }

    fn label(&self) -> String {
        format!(
            "{} :: {} ({})",
            self.zip_path.display(),
            self.psd_path_in_zip.display(),
            self.psd_file_name
        )
    }
}

impl PartialEq for FavoriteEntry {
    fn eq(&self, other: &Self) -> bool {
        self.zip_path == other.zip_path
            && self.psd_path_in_zip == other.psd_path_in_zip
            && self.mascot_scale == other.mascot_scale
    }
}

impl Eq for FavoriteEntry {}

impl Hash for FavoriteEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zip_path.hash(state);
        self.psd_path_in_zip.hash(state);
    }
}

impl From<FavoriteEntryFile> for FavoriteEntry {
    fn from(value: FavoriteEntryFile) -> Self {
        let psd_file_name = if value.psd_file_name.is_empty() {
            value
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| value.psd_path_in_zip.display().to_string())
        } else {
            value.psd_file_name
        };
        Self {
            zip_path: value.zip_path,
            psd_path_in_zip: value.psd_path_in_zip,
            psd_file_name,
            mascot_scale: sanitize_mascot_scale(value.mascot_scale),
        }
    }
}

impl FavoriteShufflePlaylist {
    pub fn new(now: Instant) -> Self {
        Self::new_with_path(favorites_path(), now)
    }

    pub(crate) fn new_with_path(favorites_path: PathBuf, now: Instant) -> Self {
        Self {
            favorites_path,
            state: FavoriteShuffleState::new(now),
        }
    }

    pub fn update(
        &mut self,
        core: &Core,
        config_path: &Path,
        current_config: &MascotConfig,
        now: Instant,
    ) -> Result<bool> {
        self.update_with_unix_timestamp(core, config_path, current_config, now, unix_timestamp())
    }

    fn update_with_unix_timestamp(
        &mut self,
        core: &Core,
        config_path: &Path,
        current_config: &MascotConfig,
        now: Instant,
        now_unix_timestamp: u64,
    ) -> Result<bool> {
        if current_config.favorite_ensemble_enabled {
            self.state
                .finish_rotation(now, current_config_key(current_config));
            return Ok(false);
        }
        if !self.state.is_due(now) {
            return Ok(false);
        }

        if suppress_rotation_for_active_display_diff(current_config) {
            let display_diff_path = current_config.display_diff_path.as_deref().unwrap();
            self.state
                .finish_rotation(now, current_config_key(current_config));
            eprintln!(
                "favorite shuffle paused while psd-viewer-tui preview is showing a non-default variation (active display diff): {}",
                display_diff_path.display()
            );
            return Ok(false);
        }
        if suppress_rotation_for_active_psd_viewer_tui(config_path, now_unix_timestamp)? {
            self.state
                .finish_rotation(now, current_config_key(current_config));
            eprintln!(
                "favorite shuffle paused while psd-viewer-tui is active: {}",
                psd_viewer_tui_activity_path(config_path).display()
            );
            return Ok(false);
        }

        let favorites = load_favorites(&self.favorites_path)?;
        let current_key = current_config_key(current_config);
        self.state.prepare_rotation(favorites, current_key.as_ref());

        while let Some(favorite) = self.state.pop_next_candidate(current_key.as_ref()) {
            let target = match favorite_target(core, favorite.clone()) {
                Ok(target) => target,
                Err(error) => {
                    eprintln!("favorite shuffle skipped '{}': {error:#}", favorite.label());
                    continue;
                }
            };
            eprintln!(
                "shuffleモード : 1分経過したので新zip psdをshuffle表示します。zip {} psd {} を拡大率{}で表示します",
                favorite.zip_path.display(),
                favorite.psd_path_in_zip.display(),
                format_scale(target.scale)
            );
            write_mascot_config(config_path, &target)?;
            self.state.finish_rotation(now, Some(favorite.key()));
            return Ok(true);
        }

        self.state.finish_rotation(now, current_key);
        Ok(false)
    }

    pub fn persist_scale_for_current_config(
        &self,
        current_config: &MascotConfig,
        scale: f32,
    ) -> Result<bool> {
        let Some(current_key) = current_config_key(current_config) else {
            return Ok(false);
        };
        let Some(mut favorites_file) = load_favorites_file(&self.favorites_path)? else {
            return Ok(false);
        };

        let sanitized_scale = sanitize_mascot_scale(Some(scale));
        let mut matched = false;
        for favorite in &mut favorites_file.favorites {
            if favorite.zip_path == current_key.zip_path
                && favorite.psd_path_in_zip == current_key.psd_path_in_zip
            {
                favorite.mascot_scale = sanitized_scale;
                matched = true;
            }
        }

        if !matched {
            return Ok(false);
        }

        save_favorites_file(&self.favorites_path, &favorites_file)?;
        eprintln!(
            "shuffleモード : + - 操作があったので、zip {} psd {} の拡大率を{}にして保存しました",
            current_key.zip_path.display(),
            current_key.psd_path_in_zip.display(),
            format_scale(sanitized_scale)
        );
        Ok(true)
    }
}

pub(crate) fn suppress_rotation_for_active_display_diff(current_config: &MascotConfig) -> bool {
    current_config.display_diff_path.is_some()
}

pub(crate) fn suppress_rotation_for_active_psd_viewer_tui(
    config_path: &Path,
    now_unix_timestamp: u64,
) -> Result<bool> {
    suppress_rotation_for_psd_viewer_tui_activity_path(
        &psd_viewer_tui_activity_path(config_path),
        now_unix_timestamp,
    )
}

pub(crate) fn suppress_rotation_for_psd_viewer_tui_activity_path(
    activity_path: &Path,
    now_unix_timestamp: u64,
) -> Result<bool> {
    if !activity_path.exists() {
        return Ok(false);
    }

    let bytes = fs::read_to_string(activity_path).with_context(|| {
        format!(
            "failed to read psd-viewer-tui activity {}",
            activity_path.display()
        )
    })?;
    let heartbeat = bytes.trim();
    if heartbeat.is_empty() {
        eprintln!(
            "favorite shuffle ignored empty psd-viewer-tui activity heartbeat {}",
            activity_path.display()
        );
        return Ok(false);
    }

    let active_at = match heartbeat.parse::<u64>() {
        Ok(active_at) => active_at,
        Err(error) => {
            eprintln!(
                "favorite shuffle ignored invalid psd-viewer-tui activity heartbeat {}: {:?} ({error})",
                activity_path.display(),
                heartbeat
            );
            return Ok(false);
        }
    };

    if active_at > now_unix_timestamp {
        eprintln!(
            "favorite shuffle ignored future psd-viewer-tui activity heartbeat {}: active_at={} now={}",
            activity_path.display(),
            active_at,
            now_unix_timestamp
        );
        return Ok(false);
    }

    Ok(now_unix_timestamp.saturating_sub(active_at) <= PSD_VIEWER_TUI_ACTIVITY_TTL.as_secs())
}
impl FavoriteShuffleState {
    fn new(now: Instant) -> Self {
        Self {
            next_rotation_at: now + FAVORITE_SHUFFLE_INTERVAL,
            last_selected_key: None,
            known_favorites: Vec::new(),
            remaining: VecDeque::new(),
            shuffle_seed: seed_from_system_time(SystemTime::now()),
        }
    }

    fn is_due(&self, now: Instant) -> bool {
        now >= self.next_rotation_at
    }

    fn prepare_rotation(
        &mut self,
        favorites: Vec<FavoriteEntry>,
        current_key: Option<&FavoriteKey>,
    ) {
        if self
            .last_selected_key
            .as_ref()
            .zip(current_key)
            .is_some_and(|(last_selected, current)| last_selected != current)
        {
            self.remaining.clear();
        }

        if self.known_favorites != favorites {
            self.known_favorites = favorites.clone();
            self.remaining.clear();
        }

        if self.remaining.is_empty() {
            self.remaining = build_playlist(&favorites, current_key, self.shuffle_seed);
            self.shuffle_seed = self.shuffle_seed.wrapping_add(1);
        }
    }

    fn pop_next_candidate(&mut self, current_key: Option<&FavoriteKey>) -> Option<FavoriteEntry> {
        while let Some(favorite) = self.remaining.pop_front() {
            if current_key.is_some_and(|current| favorite.key() == *current) {
                continue;
            }
            return Some(favorite);
        }
        None
    }

    fn finish_rotation(&mut self, now: Instant, selected_key: Option<FavoriteKey>) {
        self.next_rotation_at = now + FAVORITE_SHUFFLE_INTERVAL;
        self.last_selected_key = selected_key;
    }
}

#[cfg(test)]
impl FavoriteShufflePlaylist {
    pub(crate) fn update_with_unix_timestamp_for_test(
        &mut self,
        core: &Core,
        config_path: &Path,
        current_config: &MascotConfig,
        now: Instant,
        now_unix_timestamp: u64,
    ) -> Result<bool> {
        self.update_with_unix_timestamp(core, config_path, current_config, now, now_unix_timestamp)
    }
}

fn favorite_target(core: &Core, favorite: FavoriteEntry) -> Result<MascotTarget> {
    let rendered = core.render_png(RenderRequest {
        zip_path: favorite.zip_path.clone(),
        psd_path_in_zip: favorite.psd_path_in_zip.clone(),
        display_diff: DisplayDiff::default(),
    })?;
    Ok(MascotTarget {
        png_path: rendered.output_path,
        scale: favorite.mascot_scale,
        favorite_ensemble_scale: None,
        zip_path: favorite.zip_path,
        psd_path_in_zip: favorite.psd_path_in_zip,
        display_diff_path: None,
    })
}

fn current_config_key(current_config: &MascotConfig) -> Option<FavoriteKey> {
    if current_config.zip_path.as_os_str().is_empty()
        || current_config.psd_path_in_zip.as_os_str().is_empty()
    {
        return None;
    }
    Some(FavoriteKey {
        zip_path: current_config.zip_path.clone(),
        psd_path_in_zip: current_config.psd_path_in_zip.clone(),
    })
}

fn favorites_path() -> PathBuf {
    favorites_path_for(local_data_root())
}

pub(crate) fn favorites_path_for(data_root: &Path) -> PathBuf {
    data_root.join(FAVORITES_DIR).join(FAVORITES_FILE_NAME)
}

pub(crate) fn load_favorites(path: &Path) -> Result<Vec<FavoriteEntry>> {
    Ok(load_favorites_file(path)?
        .map(|file| sanitize_favorites(file.favorites))
        .unwrap_or_default())
}

pub(crate) fn build_playlist(
    favorites: &[FavoriteEntry],
    current_key: Option<&FavoriteKey>,
    seed: u64,
) -> VecDeque<FavoriteEntry> {
    let mut playlist = favorites.to_vec();
    let mut rng = PlaylistRng::new(seed);
    for index in (1..playlist.len()).rev() {
        let swap_index = (rng.next_u64() as usize) % (index + 1);
        playlist.swap(index, swap_index);
    }

    if playlist.len() > 1
        && current_key.is_some_and(|current| {
            playlist
                .first()
                .is_some_and(|favorite| favorite.key() == *current)
        })
    {
        if let Some(swap_index) = playlist
            .iter()
            .position(|favorite| current_key.is_none_or(|current| favorite.key() != *current))
        {
            playlist.swap(0, swap_index);
        }
    }

    VecDeque::from(playlist)
}

fn sanitize_favorites(favorites: Vec<FavoriteEntryFile>) -> Vec<FavoriteEntry> {
    let mut seen = HashSet::new();
    let mut sanitized = Vec::new();
    for (index, favorite) in favorites.into_iter().map(FavoriteEntry::from).enumerate() {
        if favorite.zip_path.as_os_str().is_empty()
            || favorite.psd_path_in_zip.as_os_str().is_empty()
        {
            eprintln!(
                "favorite shuffle dropped empty-path favorite entry at index {}",
                index
            );
            continue;
        }
        if seen.insert(favorite.key()) {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn load_favorites_file(path: &Path) -> Result<Option<FavoritesFile>> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorites {}", path.display()))?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(Some(file)),
        Err(error) => {
            eprintln!(
                "favorite shuffle ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(None)
        }
    }
}

fn save_favorites_file(path: &Path, favorites_file: &FavoritesFile) -> Result<()> {
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let toml = toml::to_string_pretty(favorites_file).context("failed to serialize favorites")?;
    fs::write(path, toml).with_context(|| format!("failed to write favorites {}", path.display()))
}

/// Keeps only finite, positive mascot scales so invalid values do not leak into shuffle restores.
fn sanitize_mascot_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn format_scale(scale: Option<f32>) -> String {
    scale
        .map(|value| format!("{value:.2}"))
        .unwrap_or("未設定".to_string())
}

fn seed_from_system_time(system_time: SystemTime) -> u64 {
    system_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

struct PlaylistRng(u64);

impl PlaylistRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(2_685_821_657_736_338_717)
    }
}
