use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::is_layer_toggle_key;

#[test]
fn layer_toggle_accepts_space_and_enter_without_modifiers() {
    assert!(is_layer_toggle_key(&KeyEvent::new(
        KeyCode::Char(' '),
        KeyModifiers::NONE
    )));
    assert!(is_layer_toggle_key(&KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE
    )));
}

#[test]
fn layer_toggle_rejects_modified_enter_and_unrelated_keys() {
    assert!(!is_layer_toggle_key(&KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::SHIFT
    )));
    assert!(!is_layer_toggle_key(&KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::NONE
    )));
}
