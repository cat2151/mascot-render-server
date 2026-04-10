use std::ffi::OsString;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};

use crate::logging::{log_control_error, log_control_info, startup_diagnostics_dir};

const EARLY_EXIT_WINDOW: Duration = Duration::from_secs(15);

pub(crate) struct SpawnedMascotRenderServer {
    pub(crate) pid: u32,
    pub(crate) command_summary: String,
    pub(crate) diagnostics_path: PathBuf,
}

pub(crate) fn spawn_mascot_render_server(config_path: &Path) -> Result<SpawnedMascotRenderServer> {
    let candidates = spawn_command_candidates(config_path)?;
    let mut last_error = None;

    for (program, args) in candidates {
        let command_summary = command_summary(&program, &args);
        let diagnostics_path = startup_diagnostics_path();
        log_control_info(format!(
            "event=server_startup stage=spawn_attempt command={command_summary} diagnostics_path={}",
            diagnostics_path.display()
        ));

        match spawn_candidate(&program, &args, &command_summary, &diagnostics_path) {
            Ok(child) => {
                let pid = child.id();
                log_control_info(format!(
                    "event=server_startup stage=spawned pid={pid} command={command_summary} diagnostics_path={}",
                    diagnostics_path.display()
                ));
                spawn_exit_logger(
                    child,
                    pid,
                    command_summary.clone(),
                    diagnostics_path.clone(),
                );
                return Ok(SpawnedMascotRenderServer {
                    pid,
                    command_summary,
                    diagnostics_path,
                });
            }
            Err(error) => {
                log_control_error(format!(
                    "event=server_startup stage=spawn_failed command={command_summary} diagnostics_path={} error={error:#}",
                    diagnostics_path.display()
                ));
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no mascot-render-server spawn command available")))
}

fn spawn_candidate(
    program: &OsString,
    args: &[OsString],
    command_summary: &str,
    diagnostics_path: &Path,
) -> Result<Child> {
    prepare_diagnostics_file(diagnostics_path, command_summary)?;
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(diagnostics_path)
        .with_context(|| {
            format!(
                "failed to open startup diagnostics stdout {}",
                diagnostics_path.display()
            )
        })?;
    let stderr = stdout.try_clone().with_context(|| {
        format!(
            "failed to clone startup diagnostics {}",
            diagnostics_path.display()
        )
    })?;

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    command.spawn().with_context(|| {
        format!(
            "failed to spawn {command_summary} with startup diagnostics {}",
            diagnostics_path.display()
        )
    })
}

fn prepare_diagnostics_file(path: &Path, command_summary: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).with_context(|| {
            format!(
                "failed to create startup diagnostics directory {}",
                parent.display()
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open startup diagnostics {}", path.display()))?;
    writeln!(file, "command={command_summary}")
        .with_context(|| format!("failed to write startup diagnostics {}", path.display()))?;
    writeln!(file)
        .with_context(|| format!("failed to finalize startup diagnostics {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush startup diagnostics {}", path.display()))?;
    Ok(())
}

fn spawn_exit_logger(child: Child, pid: u32, command_summary: String, diagnostics_path: PathBuf) {
    thread::spawn(move || {
        let started_at = Instant::now();
        match child.wait_with_output() {
            Ok(output) => {
                let elapsed = started_at.elapsed();
                let stage = if elapsed <= EARLY_EXIT_WINDOW {
                    "child_exit_during_startup"
                } else {
                    "child_exit"
                };
                let level = if output.status.success() { "INFO" } else { "ERROR" };
                let message = format!(
                    "event=server_startup stage={stage} level={level} pid={pid} status={} command={} diagnostics_path={}",
                    output.status,
                    command_summary,
                    diagnostics_path.display()
                );
                if output.status.success() {
                    log_control_info(message);
                } else {
                    log_control_error(message);
                }
            }
            Err(error) => log_control_error(format!(
                "event=server_startup stage=wait_failed pid={pid} command={command_summary} diagnostics_path={} error={error:#}",
                diagnostics_path.display()
            )),
        }
    });
}

fn startup_diagnostics_path() -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    startup_diagnostics_dir().join(format!(
        "mascot-render-server-startup-{}-{unique_suffix}.log",
        std::process::id()
    ))
}

fn command_summary(program: &OsString, args: &[OsString]) -> String {
    std::iter::once(program)
        .chain(args.iter())
        .map(shell_escape)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape(value: &OsString) -> String {
    let value = value.to_string_lossy();
    if value.contains(' ') {
        format!("\"{value}\"")
    } else {
        value.into_owned()
    }
}

fn spawn_command_candidates(config_path: &Path) -> Result<Vec<(OsString, Vec<OsString>)>> {
    let mut candidates = Vec::new();
    let sibling_binary = std::env::current_exe()
        .context("failed to resolve current executable path")?
        .with_file_name(mascot_render_server_binary_name());

    if sibling_binary.exists() {
        candidates.push((
            sibling_binary.into_os_string(),
            vec![
                OsString::from("--config"),
                config_path.as_os_str().to_os_string(),
            ],
        ));
    }

    candidates.push((
        OsString::from("cargo"),
        vec![
            OsString::from("run"),
            OsString::from("-p"),
            OsString::from("mascot-render-server"),
            OsString::from("--bin"),
            OsString::from("mascot-render-server"),
            OsString::from("--"),
            OsString::from("--config"),
            config_path.as_os_str().to_os_string(),
        ],
    ));

    Ok(candidates)
}

fn mascot_render_server_binary_name() -> &'static str {
    if cfg!(windows) {
        "mascot-render-server.exe"
    } else {
        "mascot-render-server"
    }
}
