use crossterm::event::{KeyCode, KeyModifiers};

use crate::is_layer_toggle_key;

#[test]
fn layer_toggle_key_accepts_space_and_enter_without_modifiers() {
    assert!(is_layer_toggle_key(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(is_layer_toggle_key(KeyCode::Enter, KeyModifiers::NONE));
}

#[test]
fn layer_toggle_key_rejects_other_keys_or_modified_input() {
    assert!(!is_layer_toggle_key(KeyCode::Char('j'), KeyModifiers::NONE));
    assert!(!is_layer_toggle_key(KeyCode::Enter, KeyModifiers::SHIFT));
    assert!(!is_layer_toggle_key(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL
    ));
}
