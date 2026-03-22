use std::process::Command;

use anyhow::{bail, Context, Result};

const UPDATE_GIT_URL: &str = "https://github.com/cat2151/mascot-render-server";
const UPDATE_PACKAGES: [&str; 2] = ["mascot-render-server", "psd-viewer-tui"];
#[cfg(any(target_os = "windows", test))]
const UPDATE_EXECUTABLES: [&str; 2] = ["mascot-render-server.exe", "psd-viewer-tui.exe"];

pub fn workspace_install_command() -> String {
    format!(
        "cargo install --force --git {UPDATE_GIT_URL} {}",
        UPDATE_PACKAGES.join(" ")
    )
}

#[cfg(any(target_os = "windows", test))]
pub(crate) fn update_bat_content() -> String {
    format!(
        "@echo off\r\n\
timeout /t 3 /nobreak >nul\r\n\
taskkill /F /IM {server} >nul 2>nul\r\n\
taskkill /F /IM {tui} >nul 2>nul\r\n\
{cmd}\r\n\
del \"%~f0\"\r\n",
        server = UPDATE_EXECUTABLES[0],
        tui = UPDATE_EXECUTABLES[1],
        cmd = workspace_install_command()
    )
}

pub fn run_workspace_update() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};

        let pid = std::process::id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let bat_path =
            std::env::temp_dir().join(format!("mascot_render_update_{pid}_{timestamp}.bat"));
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&bat_path)
                .with_context(|| {
                    format!("failed to create update script {}", bat_path.display())
                })?;
            file.write_all(update_bat_content().as_bytes())
                .with_context(|| format!("failed to write update script {}", bat_path.display()))?;
        }

        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(&bat_path)
            .spawn()
            .with_context(|| format!("failed to launch update script {}", bat_path.display()))?;

        println!("Launching update script: {}", bat_path.display());
        println!("The application will now exit so the update can finish.");
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("cargo")
            .args(["install", "--force", "--git", UPDATE_GIT_URL])
            .args(UPDATE_PACKAGES)
            .status()
            .context("failed to run cargo install for workspace update")?;
        if !status.success() {
            bail!("cargo install failed with status: {status}");
        }
        Ok(())
    }
}
