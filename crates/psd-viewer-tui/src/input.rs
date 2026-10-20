use crate::app;
use crossterm::event::{KeyCode, KeyModifiers};

pub(crate) fn is_layer_toggle_key(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter)
}

pub(crate) fn is_favorites_toggle_key(
    key: &crossterm::event::KeyEvent,
    favorites_visible: bool,
) -> bool {
    if key.modifiers != KeyModifiers::NONE {
        return false;
    }

    matches!(key.code, KeyCode::Char('v')) || favorites_visible && matches!(key.code, KeyCode::Esc)
}

pub(crate) fn is_overlay_close_key(
    key: &crossterm::event::KeyEvent,
    overlay_visible: bool,
) -> bool {
    overlay_visible && key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Esc)
}

pub(crate) fn is_overlay_confirm_key(
    key: &crossterm::event::KeyEvent,
    overlay_visible: bool,
) -> bool {
    overlay_visible && key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Enter)
}

pub(crate) fn is_favorite_save_key(
    key: &crossterm::event::KeyEvent,
    focus: app::FocusPane,
    favorites_visible: bool,
) -> bool {
    key.modifiers == KeyModifiers::NONE
        && matches!(key.code, KeyCode::Char('f'))
        && !favorites_visible
        && matches!(focus, app::FocusPane::Library | app::FocusPane::Layer)
}

pub(crate) fn is_favorite_ensemble_toggle_key(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Char('e'))
}
