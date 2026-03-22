use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_workspace_root).as_path()
}

pub(crate) fn workspace_log_root() -> PathBuf {
    workspace_root().join("log")
}

fn resolve_workspace_root() -> PathBuf {
    workspace_root_from_current_dir()
        .or_else(workspace_root_from_current_exe)
        .unwrap_or_else(fallback_workspace_root)
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
        .find(|candidate| {
            candidate.join("Cargo.toml").is_file()
                && candidate.join("crates/tui-sixel-preview/Cargo.toml").is_file()
        })
        .map(Path::to_path_buf)
}
