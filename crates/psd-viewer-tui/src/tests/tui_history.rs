use std::fs;

use crate::tui_history::{load_tui_history, save_tui_history, TuiHistory};
use mascot_render_core::workspace_cache_root;

#[test]
fn tui_history_round_trips_selected_layer_cursor() {
    let cache_root = workspace_cache_root().join("test-tui-history");
    let _ = fs::remove_dir_all(&cache_root);

    save_tui_history(
        &cache_root,
        &TuiHistory {
            selected_node: Some(17),
        },
    )
    .expect("should write TUI history");

    let loaded = load_tui_history(&cache_root).expect("should read TUI history");
    assert_eq!(
        loaded,
        TuiHistory {
            selected_node: Some(17),
        }
    );
}

#[test]
fn invalid_tui_history_falls_back_to_default() {
    let cache_root = workspace_cache_root().join("test-tui-history-invalid");
    let _ = fs::remove_dir_all(&cache_root);
    fs::create_dir_all(&cache_root).expect("should create temp directory");
    fs::write(cache_root.join("history_tui.json"), "{ invalid json")
        .expect("should seed invalid history");

    let loaded = load_tui_history(&cache_root).expect("invalid history should fall back");
    assert_eq!(loaded, TuiHistory::default());
}
