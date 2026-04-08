use std::process::Command;

#[cfg(not(target_os = "windows"))]
use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use cat_self_update_lib::{check_remote_commit, CheckResult};

const UPDATE_REPO_OWNER: &str = "cat2151";
const UPDATE_REPO_NAME: &str = "mascot-render-server";
const UPDATE_BRANCH: &str = "main";
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
if %ERRORLEVEL% neq 0 (\r\n\
  echo Update failed with error %ERRORLEVEL%.\r\n\
  echo The update script will not be deleted so you can inspect or rerun it.\r\n\
  pause\r\n\
  exit /b %ERRORLEVEL%\r\n\
)\r\n\
del \"%~f0\"\r\n",
        server = UPDATE_EXECUTABLES[0],
        tui = UPDATE_EXECUTABLES[1],
        cmd = workspace_install_command()
    )
}

type CheckRemoteCommitFn =
    fn(&str, &str, &str, &str) -> std::result::Result<CheckResult, Box<dyn std::error::Error>>;

pub(crate) fn check_workspace_update_with(
    build_commit_hash: &str,
    checker: CheckRemoteCommitFn,
) -> Result<String> {
    checker(
        UPDATE_REPO_OWNER,
        UPDATE_REPO_NAME,
        UPDATE_BRANCH,
        build_commit_hash,
    )
    .map(|result| result.to_string())
    .map_err(|error| anyhow!("failed to check for workspace update: {error}"))
}

pub fn check_workspace_update(build_commit_hash: &str) -> Result<String> {
    check_workspace_update_with(build_commit_hash, check_remote_commit)
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
        Ok(())
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
