use std::error::Error;
use std::fmt;

use cat_self_update_lib::CheckResult;

use crate::workspace_update::{
    check_workspace_update_with, update_bat_content, workspace_install_command,
};

const FAKE_CHECK_ERROR_MESSAGE: &str = "network down";

#[test]
fn workspace_install_command_matches_readme_install_command() {
    assert_eq!(
        workspace_install_command(),
        "cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui"
    );
}

#[test]
fn windows_update_bat_stops_both_binaries_and_reinstalls_workspace() {
    let bat = update_bat_content();

    assert!(bat.contains("taskkill /F /IM mascot-render-server.exe >nul 2>nul"));
    assert!(bat.contains("taskkill /F /IM psd-viewer-tui.exe >nul 2>nul"));
    assert!(bat.contains(&workspace_install_command()));
    assert!(bat.contains("if %ERRORLEVEL% neq 0 ("));
    assert!(bat.contains("The update script will not be deleted so you can inspect or rerun it."));
    assert!(bat.contains("pause"));
    assert!(bat.contains("del \"%~f0\""));
}

fn fake_check_success(
    owner: &str,
    repo: &str,
    branch: &str,
    embedded_hash: &str,
) -> Result<CheckResult, Box<dyn Error>> {
    assert_eq!(owner, "cat2151");
    assert_eq!(repo, "mascot-render-server");
    assert_eq!(branch, "main");
    assert_eq!(embedded_hash, "embedded123");

    Ok(CheckResult {
        embedded_hash: embedded_hash.to_owned(),
        remote_hash: embedded_hash.to_owned(),
    })
}

#[derive(Debug)]
struct FakeCheckError;

impl fmt::Display for FakeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{FAKE_CHECK_ERROR_MESSAGE}")
    }
}

impl Error for FakeCheckError {}

fn fake_check_failure(
    _owner: &str,
    _repo: &str,
    _branch: &str,
    _embedded_hash: &str,
) -> Result<CheckResult, Box<dyn Error>> {
    Err(Box::new(FakeCheckError))
}

#[test]
fn check_workspace_update_uses_workspace_repo_arguments() {
    let result = check_workspace_update_with("embedded123", fake_check_success)
        .expect("workspace update check should succeed");

    assert_eq!(
        result,
        "embedded: embedded123\nremote: embedded123\nresult: up-to-date"
    );
}

#[test]
fn check_workspace_update_adds_context_to_errors() {
    let error =
        check_workspace_update_with("embedded123", fake_check_failure).expect_err("should fail");

    assert_eq!(
        error.to_string(),
        format!("failed to check for workspace update: {FAKE_CHECK_ERROR_MESSAGE}")
    );
}
