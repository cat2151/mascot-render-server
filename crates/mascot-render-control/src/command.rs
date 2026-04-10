use std::path::PathBuf;

use mascot_render_client::MotionTimelineRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MascotControlCommand {
    Show,
    Hide,
    ChangeSkin(PathBuf),
    PlayTimeline(MotionTimelineRequest),
}
