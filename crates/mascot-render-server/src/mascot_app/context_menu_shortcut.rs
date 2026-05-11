use eframe::egui::{InputState, Key, Modifiers};
use mascot_render_core::MascotEnsembleMode;
use mascot_render_protocol::PlacementMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlacementContextMenuAction {
    SetEnsembleMode(MascotEnsembleMode),
    SetPlacementMode(PlacementMode),
    Quit,
}

const PLACEMENT_CONTEXT_MENU_SHORTCUTS: &[(Key, PlacementContextMenuAction)] = &[
    (
        Key::Num1,
        PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::SingleCharacter),
    ),
    (
        Key::F,
        PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::Favorite),
    ),
    (
        Key::V,
        PlacementContextMenuAction::SetEnsembleMode(MascotEnsembleMode::Vpt),
    ),
    (
        Key::P,
        PlacementContextMenuAction::SetPlacementMode(PlacementMode::PerPsd),
    ),
    (
        Key::S,
        PlacementContextMenuAction::SetPlacementMode(PlacementMode::SharedVisualSize),
    ),
    (Key::Q, PlacementContextMenuAction::Quit),
];

pub(crate) fn placement_context_menu_action_for_input(
    input: &InputState,
) -> Option<PlacementContextMenuAction> {
    if shortcut_modifiers_blocked(input.modifiers) {
        return None;
    }

    PLACEMENT_CONTEXT_MENU_SHORTCUTS
        .iter()
        .find_map(|(key, action)| input.key_pressed(*key).then_some(*action))
}

fn shortcut_modifiers_blocked(modifiers: Modifiers) -> bool {
    modifiers.alt || modifiers.ctrl || modifiers.command || modifiers.mac_cmd
}

#[cfg(test)]
pub(crate) fn placement_context_menu_action_for_key_for_test(
    key: Key,
    modifiers: Modifiers,
) -> Option<PlacementContextMenuAction> {
    if shortcut_modifiers_blocked(modifiers) {
        return None;
    }

    PLACEMENT_CONTEXT_MENU_SHORTCUTS
        .iter()
        .find_map(|(shortcut_key, action)| (*shortcut_key == key).then_some(*action))
}
