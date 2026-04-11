use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::favorite_shuffle::FavoriteEntry;
use mascot_render_core::{
    workspace_cache_root, AlwaysBendConfig, BounceAnimationConfig, IdleSinkAnimationConfig,
    MascotConfig, SquashBounceAnimationConfig,
};

pub(super) fn favorite(
    zip_path: &str,
    psd_path_in_zip: &str,
    psd_file_name: &str,
) -> FavoriteEntry {
    FavoriteEntry {
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        psd_file_name: psd_file_name.to_string(),
        mascot_scale: None,
    }
}

pub(super) fn mascot_config(zip_path: &str, psd_path_in_zip: &str) -> MascotConfig {
    MascotConfig {
        png_path: PathBuf::from("/workspace/render.png"),
        scale: Some(1.0),
        favorite_ensemble_scale: None,
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        display_diff_path: None,
        always_idle_sink_enabled: false,
        always_bend: AlwaysBendConfig::default(),
        favorite_ensemble_enabled: false,
        bounce: BounceAnimationConfig::default(),
        squash_bounce: SquashBounceAnimationConfig::default(),
        always_idle_sink: IdleSinkAnimationConfig::default_for_always_bouncing(),
    }
}

pub(super) fn create_invalid_favorites_path(favorites_path: &Path) {
    let favorites_dir = favorites_path
        .parent()
        .expect("favorites path should have a parent directory");
    fs::create_dir_all(favorites_dir).expect("should create favorites directory");
    fs::create_dir(favorites_path).expect(
        "should create directory at favorites file path to simulate invalid favorites file",
    );
}

pub(super) fn unique_test_root(prefix: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    workspace_cache_root().join(format!("{prefix}-{unique_suffix}-{id}"))
}

pub(super) fn replace_path_with_directory(path: &Path) {
    if path.is_dir() {
        fs::remove_dir_all(path).expect("should remove stale fixture directory");
    } else {
        fs::remove_file(path).ok();
    }
    fs::create_dir_all(
        path.parent()
            .expect("fixture path should have a parent directory"),
    )
    .expect("should create fixture parent directory");
    fs::create_dir(path)
        .expect("should create directory at fixture file path to simulate unreadable file");
}

/// RAII guard that removes a temporary test fixture path on drop.
pub(super) struct TestFixtureCleanup(pub(super) PathBuf);

impl Drop for TestFixtureCleanup {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}
