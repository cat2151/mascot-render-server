use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::is_favorite_ensemble_toggle_key;
use mascot_render_core::{
    load_favorite_ensemble_enabled, mascot_runtime_state_path, workspace_cache_root,
};

#[test]
fn favorite_ensemble_toggle_accepts_plain_e_only() {
    let plain_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    let upper_e = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT);
    let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);

    assert!(is_favorite_ensemble_toggle_key(&plain_e));
    assert!(!is_favorite_ensemble_toggle_key(&upper_e));
    assert!(!is_favorite_ensemble_toggle_key(&ctrl_e));
}

#[test]
fn favorite_ensemble_toggle_updates_config_and_status_message() {
    let root = workspace_cache_root().join("test-psd-viewer-favorite-ensemble-toggle");
    let config_path = root.join("mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(&root).expect("should create temp directory");

    let mut app = App::loading(None);

    assert!(!app
        .toggle_favorite_ensemble_enabled_for_test(&config_path)
        .expect("should toggle favorite ensemble without runtime sync"));
    assert!(load_favorite_ensemble_enabled(&config_path).expect("config should load"));
    assert_eq!(
        line_text(app.log_lines()[0].clone()),
        "Message: favorite_ensemble_enabled = true"
    );

    assert!(!app
        .toggle_favorite_ensemble_enabled_for_test(&config_path)
        .expect("should toggle favorite ensemble without runtime sync"));
    assert!(!load_favorite_ensemble_enabled(&config_path).expect("config should load"));
    assert_eq!(
        line_text(app.log_lines()[0].clone()),
        "Message: favorite_ensemble_enabled = false"
    );
}

#[test]
fn favorite_ensemble_toggle_with_runtime_sync_returns_false_without_selected_preview() {
    let root = workspace_cache_root().join("test-psd-viewer-favorite-ensemble-toggle-no-preview");
    let config_path = root.join("mascot-render-server.toml");
    let runtime_state_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&runtime_state_path);
    fs::create_dir_all(&root).expect("should create temp directory");

    let mut app = App::loading(None);

    assert!(!app
        .toggle_favorite_ensemble_enabled_with_sync_for_test(&config_path)
        .expect("should toggle without runtime sync target"));
    assert!(load_favorite_ensemble_enabled(&config_path).expect("config should load"));
}

fn line_text(line: ratatui::text::Line<'static>) -> String {
    line.spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect()
}
