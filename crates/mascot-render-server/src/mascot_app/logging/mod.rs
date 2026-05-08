use eframe::egui::Pos2;
use mascot_render_protocol::PlacementAnchorKind;
use mascot_render_server::window_history::ViewportInfo;
use mascot_render_server::MascotWindowLayout;

mod app;
mod control_command;
mod formatting;
mod scale;

pub(crate) use control_command::{
    change_character_stage_message, change_character_success_message, clear_rendered_skin_path,
    preview_target_failure_message, preview_target_stage_message, preview_target_success_message,
    run_change_character_stage, should_log_rendered_skin,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ScaleChangeTrigger {
    Keyboard,
    MouseWheel { raw_scroll_delta_y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScaleLayoutChange {
    pub selected_anchor_kind: PlacementAnchorKind,
    pub previous_layout: MascotWindowLayout,
    pub next_layout: MascotWindowLayout,
    pub viewport_info: Option<ViewportInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefreshWindowLayoutDiagnostics {
    pub selected_anchor_kind: PlacementAnchorKind,
    pub previous_layout: MascotWindowLayout,
    pub next_layout: MascotWindowLayout,
    pub viewport_info: Option<ViewportInfo>,
    pub preserved_anchor_position: Option<Pos2>,
    pub next_inner_origin: Option<Pos2>,
    pub next_outer_position: Option<Pos2>,
}

#[cfg(test)]
pub(crate) use control_command::change_character_failure_message as change_character_failure_message_for_test;
#[cfg(test)]
pub(crate) use control_command::change_character_stage_message as change_character_stage_message_for_test;
#[cfg(test)]
pub(crate) use control_command::change_character_success_message as change_character_success_message_for_test;
#[cfg(test)]
pub(crate) use control_command::clear_rendered_skin_path as clear_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use control_command::record_rendered_skin_path as record_rendered_skin_path_for_test;
#[cfg(test)]
pub(crate) use control_command::rendered_skin_message as rendered_skin_message_for_test;
#[cfg(test)]
pub(crate) use control_command::should_log_rendered_skin as should_log_rendered_skin_for_test;
#[cfg(test)]
pub(crate) use scale::hot_reload_context_message as hot_reload_context_message_for_test;
#[cfg(test)]
pub(crate) use scale::refresh_window_layout_message as refresh_window_layout_message_for_test;
#[cfg(test)]
pub(crate) use scale::reloaded_scale_message as reloaded_scale_message_for_test;
#[cfg(test)]
pub(crate) use scale::scale_change_message as scale_change_message_for_test;
#[cfg(test)]
pub(crate) use scale::scale_layout_change_message as scale_layout_change_message_for_test;
#[cfg(test)]
pub(crate) use RefreshWindowLayoutDiagnostics as RefreshWindowLayoutDiagnosticsForTest;
#[cfg(test)]
pub(crate) use ScaleChangeTrigger as ScaleChangeTriggerForTest;
#[cfg(test)]
pub(crate) use ScaleLayoutChange as ScaleLayoutChangeForTest;
