mod command;
mod status;
mod store;

#[cfg(test)]
mascot_render_test_support::install_test_data_root!();

#[cfg(test)]
mod tests;

pub use command::{
    validate_motion_timeline_request, validate_preview_target_request, ChangeCharacterRequest,
    MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep, PreviewTargetRequest,
};
pub use status::{
    now_unix_ms, PlacementAnchorKind, PlacementAnchorPlan, PlacementAnchorPlanTarget,
    PlacementAnchorPolicy, PlacementAnchorPositions, PlacementMode, PlacementTargetScope,
    ScreenRectPx, ServerCommandKind, ServerCommandStage, ServerCommandStatus, ServerLifecyclePhase,
    ServerMotionStatus, ServerPlacementStatus, ServerStatusSnapshot, ServerWindowStatus,
    ServerWorkStatus, SharedVisualSizePolicy, VisualSizePx,
};
pub use store::ServerStatusStore;
