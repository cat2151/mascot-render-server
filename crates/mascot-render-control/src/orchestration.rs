use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use mascot_render_client::{
    change_skin_mascot_render_server, mascot_render_server_address,
    mascot_render_server_healthcheck_at, play_timeline_mascot_render_server,
    show_mascot_render_server,
};
use mascot_render_protocol::MotionTimelineRequest;

use crate::logging::{log_control_error, log_control_info};
use crate::spawn::{spawn_mascot_render_server, SpawnExitEvent, SpawnedMascotRenderServer};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const HEALTHCHECK_RETRY_INTERVAL: Duration = Duration::from_millis(100);
const STARTUP_DIAGNOSTICS_TAIL_CHARS: usize = 2_000;

pub fn ensure_mascot_render_server_visible(config_path: &Path) -> Result<()> {
    ensure_mascot_render_server_running(config_path)?;
    show_mascot_render_server()
}

pub fn ensure_mascot_render_server_running(config_path: &Path) -> Result<()> {
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

    Ok(())
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
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    let mut last_error = None;

    while Instant::now() < deadline {
        if let Some(exit) = spawned.try_recv_exit() {
            return report_startup_child_exit(address, spawned, exit, last_error.as_ref());
        }

        match mascot_render_server_healthcheck_at(address) {
            Ok(()) => {
                log_control_info(format!(
                    "event=server_startup stage=healthy address={address} pid={} command={} diagnostics_path={}",
                    spawned.pid,
                    spawned.command_summary,
                    spawned.diagnostics_path.display()
                ));
                return Ok(());
            }
            Err(error) => {
                last_error = Some(error);
            }
        }

        if let Some(exit) = spawned.try_recv_exit() {
            return report_startup_child_exit(address, spawned, exit, last_error.as_ref());
        }

        thread::sleep(HEALTHCHECK_RETRY_INTERVAL);
    }

    log_control_error(format!(
        "event=server_startup stage=healthcheck_timeout address={address} pid={} command={} diagnostics_path={} error={}",
        spawned.pid,
        spawned.command_summary,
        spawned.diagnostics_path.display(),
        last_error
            .as_ref()
            .map(|error| format!("{error:#}"))
            .unwrap_or_else(|| "startup timed out before first healthcheck".to_string())
    ));

    Err(last_error.unwrap_or_else(|| anyhow!("startup timed out before first healthcheck")))
        .with_context(|| {
            format!(
                "mascot-render-server did not become healthy after spawn: pid={} command={} diagnostics_path={}{}",
                spawned.pid,
                spawned.command_summary,
                spawned.diagnostics_path.display(),
                startup_diagnostics_context(&spawned.diagnostics_path)
            )
        })
}

fn report_startup_child_exit(
    address: std::net::SocketAddr,
    spawned: &SpawnedMascotRenderServer,
    exit: SpawnExitEvent,
    last_error: Option<&anyhow::Error>,
) -> Result<()> {
    match exit {
        SpawnExitEvent::Exited { status, elapsed } => {
            let message = format!(
                "event=server_startup stage=child_exit_before_healthcheck address={address} pid={} status={} elapsed_ms={} command={} diagnostics_path={} last_healthcheck_error={}",
                spawned.pid,
                status,
                elapsed.as_millis(),
                spawned.command_summary,
                spawned.diagnostics_path.display(),
                last_error
                    .map(|error| format!("{error:#}"))
                    .unwrap_or_else(|| "-".to_string())
            );
            if status.success() {
                log_control_info(message);
            } else {
                log_control_error(message);
            }
            bail!(
                "mascot-render-server exited before healthcheck succeeded: pid={} status={} elapsed_ms={} command={} diagnostics_path={}{}",
                spawned.pid,
                status,
                elapsed.as_millis(),
                spawned.command_summary,
                spawned.diagnostics_path.display(),
                startup_diagnostics_context(&spawned.diagnostics_path)
            );
        }
        SpawnExitEvent::WaitFailed(error) => {
            log_control_error(format!(
                "event=server_startup stage=child_wait_failed_before_healthcheck address={address} pid={} command={} diagnostics_path={} error={error} last_healthcheck_error={}",
                spawned.pid,
                spawned.command_summary,
                spawned.diagnostics_path.display(),
                last_error
                    .map(|error| format!("{error:#}"))
                    .unwrap_or_else(|| "-".to_string())
            ));
            bail!(
                "failed to wait for mascot-render-server startup child: pid={} command={} diagnostics_path={} error={}{}",
                spawned.pid,
                spawned.command_summary,
                spawned.diagnostics_path.display(),
                error,
                startup_diagnostics_context(&spawned.diagnostics_path)
            );
        }
    }
}

fn startup_diagnostics_context(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => String::new(),
        Ok(text) => format!(
            "\nstartup diagnostics tail:\n{}",
            tail_chars(&text, STARTUP_DIAGNOSTICS_TAIL_CHARS)
        ),
        Err(error) => format!("\nfailed to read startup diagnostics: {error:#}"),
    }
}

fn tail_chars(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    text.chars().skip(char_count - max_chars).collect()
}

fn startup_singleflight_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
pub(crate) fn startup_diagnostics_context_for_test(path: &Path) -> String {
    startup_diagnostics_context(path)
}

#[cfg(test)]
pub(crate) fn tail_chars_for_test(text: &str, max_chars: usize) -> String {
    tail_chars(text, max_chars)
}
