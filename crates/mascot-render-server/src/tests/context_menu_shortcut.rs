use eframe::egui::{Key, Modifiers};
use mascot_render_protocol::PlacementMode;

use crate::mascot_app::{
    placement_context_menu_action_for_key_for_test, PlacementContextMenuAction,
};

#[test]
fn placement_context_menu_shortcuts_map_letters_to_actions() {
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::F, Modifiers::NONE),
        Some(PlacementContextMenuAction::ToggleFavoriteEnsemble)
    );
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::P, Modifiers::NONE),
        Some(PlacementContextMenuAction::SetPlacementMode(
            PlacementMode::PerPsd
        ))
    );
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::S, Modifiers::NONE),
        Some(PlacementContextMenuAction::SetPlacementMode(
            PlacementMode::SharedVisualSize
        ))
    );
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::Q, Modifiers::NONE),
        Some(PlacementContextMenuAction::Quit)
    );
}

#[test]
fn placement_context_menu_shortcuts_ignore_unassigned_keys() {
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::A, Modifiers::NONE),
        None
    );
}

#[test]
fn placement_context_menu_shortcuts_allow_shift_and_ignore_command_modifiers() {
    let mut shift = Modifiers::NONE;
    shift.shift = true;
    assert_eq!(
        placement_context_menu_action_for_key_for_test(Key::Q, shift),
        Some(PlacementContextMenuAction::Quit)
    );

    for modifiers in command_modifiers() {
        assert_eq!(
            placement_context_menu_action_for_key_for_test(Key::Q, modifiers),
            None
        );
    }
}

fn command_modifiers() -> [Modifiers; 4] {
    let mut alt = Modifiers::NONE;
    alt.alt = true;
    let mut ctrl = Modifiers::NONE;
    ctrl.ctrl = true;
    let mut command = Modifiers::NONE;
    command.command = true;
    let mut mac_cmd = Modifiers::NONE;
    mac_cmd.mac_cmd = true;
    [alt, ctrl, command, mac_cmd]
}
