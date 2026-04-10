use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use mascot_render_client::{
    change_skin_mascot_render_server, mascot_render_server_address,
    mascot_render_server_healthcheck_at, play_timeline_mascot_render_server,
    show_mascot_render_server, wait_for_mascot_render_server_healthcheck_at, MotionTimelineRequest,
};

use crate::spawn::spawn_mascot_render_server;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

pub fn ensure_mascot_render_server_visible(config_path: &Path) -> Result<()> {
    let address = mascot_render_server_address();
    if mascot_render_server_healthcheck_at(address).is_err() {
        spawn_mascot_render_server(config_path)?;
        wait_for_mascot_render_server_healthcheck_at(address, STARTUP_TIMEOUT)?;
    }

    show_mascot_render_server()
}

pub fn sync_mascot_render_server_preview(
    config_path: &Path,
    png_path: Option<&Path>,
) -> Result<()> {
    let Some(png_path) = png_path else {
        return Ok(());
    };

    ensure_mascot_render_server_visible(config_path)?;
    change_skin_mascot_render_server(png_path)
}

pub fn play_mascot_render_server_timeline(
    config_path: &Path,
    request: &MotionTimelineRequest,
) -> Result<()> {
    ensure_mascot_render_server_visible(config_path)?;
    play_timeline_mascot_render_server(request)
}
