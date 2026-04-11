use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use mascot_render_client::{
    change_skin_mascot_render_server, mascot_render_server_address,
    mascot_render_server_healthcheck_at, play_timeline_mascot_render_server,
    show_mascot_render_server, wait_for_mascot_render_server_healthcheck_at,
};
use mascot_render_protocol::MotionTimelineRequest;

use crate::logging::{log_control_error, log_control_info};
use crate::spawn::{spawn_mascot_render_server, SpawnedMascotRenderServer};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

pub fn ensure_mascot_render_server_visible(config_path: &Path) -> Result<()> {
    let _startup_guard = startup_singleflight_lock()
        .lock()
        .map_err(|_| anyhow!("mascot-render-server startup lock was poisoned"))?;
    let address = mascot_render_server_address();

    if let Err(error) = mascot_render_server_healthcheck_at(address) {
        log_control_info(format!(
            "event=server_startup stage=healthcheck_failed address={address} error={error:#}"
        ));
        let spawned = spawn_mascot_render_server(config_path)?;
        wait_for_startup(address, &spawned)?;
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

fn wait_for_startup(
    address: std::net::SocketAddr,
    spawned: &SpawnedMascotRenderServer,
) -> Result<()> {
    match wait_for_mascot_render_server_healthcheck_at(address, STARTUP_TIMEOUT) {
        Ok(()) => {
            log_control_info(format!(
                "event=server_startup stage=healthy address={address} pid={} command={} diagnostics_path={}",
                spawned.pid,
                spawned.command_summary,
                spawned.diagnostics_path.display()
            ));
            Ok(())
        }
        Err(error) => {
            log_control_error(format!(
                "event=server_startup stage=healthcheck_timeout address={address} pid={} command={} diagnostics_path={} error={error:#}",
                spawned.pid,
                spawned.command_summary,
                spawned.diagnostics_path.display()
            ));
            Err(error).with_context(|| {
                format!(
                    "mascot-render-server did not become healthy after spawn: pid={} command={} diagnostics_path={}",
                    spawned.pid,
                    spawned.command_summary,
                    spawned.diagnostics_path.display()
                )
            })
        }
    }
}

fn startup_singleflight_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
