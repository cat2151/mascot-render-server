use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

pub(crate) fn spawn_mascot_render_server(config_path: &Path) -> Result<()> {
    let candidates = spawn_command_candidates(config_path)?;
    let mut last_error = None;

    for (program, args) in candidates {
        let mut command = Command::new(&program);
        command
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match command.spawn() {
            Ok(_child) => return Ok(()),
            Err(error) => {
                last_error = Some(anyhow!(
                    "failed to spawn {:?} {:?}: {}",
                    program,
                    args,
                    error
                ));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no mascot-render-server spawn command available")))
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
