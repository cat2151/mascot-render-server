use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mascot_render_core::{
    local_data_root, write_mascot_config, Core, DisplayDiff, MascotConfig, MascotTarget,
    RenderRequest,
};
use serde::Deserialize;

const FAVORITES_DIR: &str = "favorites";
const FAVORITES_FILE_NAME: &str = "psd-viewer-tui.toml";
const FAVORITES_FILE_VERSION: u32 = 1;

pub const FAVORITE_SHUFFLE_INTERVAL: Duration = Duration::from_secs(60);

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FavoriteKey {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
}

#[derive(Debug, Deserialize)]
struct FavoriteEntryFile {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    #[serde(default)]
    psd_file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct FavoritesFile {
    version: u32,
    favorites: Vec<FavoriteEntryFile>,
}

impl Default for FavoritesFile {
    fn default() -> Self {
        Self {
            version: FAVORITES_FILE_VERSION,
            favorites: Vec::new(),
        }
    }
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
        self.zip_path == other.zip_path && self.psd_path_in_zip == other.psd_path_in_zip
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

        let favorites = load_favorites(&self.favorites_path)?;
        let current_key = current_config_key(current_config);
        self.state.prepare_rotation(favorites, current_key.as_ref());

        while let Some(favorite) = self.state.pop_next_candidate(current_key.as_ref()) {
            let target = match favorite_target(core, current_config, favorite.clone()) {
                Ok(target) => target,
                Err(error) => {
                    eprintln!("favorite shuffle skipped '{}': {error:#}", favorite.label());
                    continue;
                }
            };
            write_mascot_config(config_path, &target)?;
            self.state.finish_rotation(now, Some(favorite.key()));
            return Ok(true);
        }

        self.state.finish_rotation(now, current_key);
        Ok(false)
    }
}

pub(crate) fn suppress_rotation_for_active_display_diff(current_config: &MascotConfig) -> bool {
    current_config.display_diff_path.is_some()
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

fn favorite_target(
    core: &Core,
    current_config: &MascotConfig,
    favorite: FavoriteEntry,
) -> Result<MascotTarget> {
    let rendered = core.render_png(RenderRequest {
        zip_path: favorite.zip_path.clone(),
        psd_path_in_zip: favorite.psd_path_in_zip.clone(),
        display_diff: DisplayDiff::default(),
    })?;
    Ok(MascotTarget {
        png_path: rendered.output_path,
        scale: current_config.scale,
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
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read favorites {}", path.display()))?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) if file.version == FAVORITES_FILE_VERSION => {
            Ok(sanitize_favorites(file.favorites))
        }
        Ok(file) => {
            eprintln!(
                "favorite shuffle ignored unsupported favorites cache version {} at {}",
                file.version,
                path.display()
            );
            Ok(Vec::new())
        }
        Err(error) => {
            eprintln!(
                "favorite shuffle ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(Vec::new())
        }
    }
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
