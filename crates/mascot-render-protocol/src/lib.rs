mod command;
mod status;
mod store;

#[cfg(test)]
mascot_render_test_support::install_test_data_root!();

#[cfg(test)]
mod tests;

pub use command::{
    validate_motion_timeline_request, ChangeCharacterRequest, MotionTimelineKind,
    MotionTimelineRequest, MotionTimelineStep,
};
pub use status::{
    now_unix_ms, ServerCommandKind, ServerCommandStage, ServerCommandStatus, ServerLifecyclePhase,
    ServerMotionStatus, ServerStatusSnapshot, ServerWindowStatus, ServerWorkStatus,
};
pub use store::ServerStatusStore;
