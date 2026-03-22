use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const WORKSPACE_ROOT_ENV: &str = "ZUNDAMON_PSD_VIEWER_ROOT";

pub fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_workspace_root).as_path()
}

pub fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root().join(path)
    }
}

pub fn workspace_cache_root() -> PathBuf {
    workspace_path("cache")
}

pub fn workspace_log_root() -> PathBuf {
    workspace_path("log")
}

pub fn workspace_relative_display_path(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resolve_workspace_root() -> PathBuf {
    workspace_root_from_env()
        .or_else(workspace_root_from_current_dir)
        .or_else(workspace_root_from_current_exe)
        .unwrap_or_else(fallback_workspace_root)
}

fn workspace_root_from_env() -> Option<PathBuf> {
    let configured = std::env::var_os(WORKSPACE_ROOT_ENV)?;
    let path = PathBuf::from(configured);
    is_workspace_root(&path).then_some(path)
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

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|candidate| is_workspace_root(candidate))
        .map(Path::to_path_buf)
}

fn is_workspace_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("crates/mascot-render-core/Cargo.toml").is_file()
}

