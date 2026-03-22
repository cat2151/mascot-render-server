use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const LOCAL_DATA_ROOT_ENV: &str = "MASCOT_RENDER_SERVER_DATA_ROOT";
const LOCAL_DATA_DIR_NAME: &str = "mascot-render-server";
const WORKSPACE_ROOT_ENV: &str = "ZUNDAMON_PSD_VIEWER_ROOT";

pub fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_workspace_root).as_path()
}

pub fn local_data_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_local_data_root).as_path()
}

pub fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root().join(path)
    }
}

pub fn local_data_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        local_data_root().join(path)
    }
}

pub fn workspace_cache_root() -> PathBuf {
    local_data_path("cache")
}

pub fn workspace_log_root() -> PathBuf {
    local_data_path("log")
}

pub fn workspace_relative_display_path(path: &Path) -> String {
    relative_to_known_root(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn relative_to_known_root<'a>(path: &'a Path) -> &'a Path {
    path.strip_prefix(local_data_root())
        .or_else(|_| path.strip_prefix(workspace_root()))
        .unwrap_or(path)
}

fn resolve_workspace_root() -> PathBuf {
    workspace_root_from_env()
        .or_else(workspace_root_from_current_dir)
        .or_else(workspace_root_from_current_exe)
        .unwrap_or_else(fallback_workspace_root)
}

fn resolve_local_data_root() -> PathBuf {
    local_data_root_from_env()
        .or_else(local_data_root_from_windows_appdata)
        .unwrap_or_else(fallback_local_data_root)
}

fn workspace_root_from_env() -> Option<PathBuf> {
    let configured = std::env::var_os(WORKSPACE_ROOT_ENV)?;
    let path = PathBuf::from(configured);
    is_workspace_root(&path).then_some(path)
}

fn local_data_root_from_env() -> Option<PathBuf> {
    let configured = std::env::var_os(LOCAL_DATA_ROOT_ENV)?;
    let path = PathBuf::from(configured);
    if path.as_os_str().is_empty() {
        None
    } else if path.is_absolute() {
        Some(path)
    } else {
        Some(workspace_root().join(path))
    }
}

fn workspace_root_from_current_dir() -> Option<PathBuf> {
    find_workspace_root(std::env::current_dir().ok()?.as_path())
}

fn workspace_root_from_current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    find_workspace_root(exe.parent()?)
}

fn fallback_workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

#[cfg(target_os = "windows")]
fn local_data_root_from_windows_appdata() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let path = PathBuf::from(base);
    (!path.as_os_str().is_empty()).then_some(path.join(LOCAL_DATA_DIR_NAME))
}

#[cfg(not(target_os = "windows"))]
fn local_data_root_from_windows_appdata() -> Option<PathBuf> {
    None
}

fn fallback_local_data_root() -> PathBuf {
    workspace_root().to_path_buf()
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|candidate| is_workspace_root(candidate))
        .map(Path::to_path_buf)
}

fn is_workspace_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("crates/mascot-render-core/Cargo.toml").is_file()
}
