use std::error::Error;

use anyhow::{anyhow, Context, Result};
use cat_self_update_lib::{check_remote_commit, self_update, CheckResult};

const UPDATE_REPO_OWNER: &str = "cat2151";
const UPDATE_REPO_NAME: &str = "mascot-render-server";
const UPDATE_BRANCH: &str = "main";
const UPDATE_GIT_URL: &str = "https://github.com/cat2151/mascot-render-server";
const UPDATE_PACKAGES: [&str; 2] = ["mascot-render-server", "psd-viewer-tui"];
const UPDATE_BINARIES: [&str; 2] = ["mascot-render-server", "psd-viewer-tui"];

pub fn workspace_install_command() -> String {
    format!(
        "cargo install --force --git {UPDATE_GIT_URL} {}",
        UPDATE_PACKAGES.join(" ")
    )
}

/// Function signature for checking the remote commit of the workspace repository.
/// Parameters are `(owner, repo, branch, embedded_hash)`.
type LibResult<T> = std::result::Result<T, Box<dyn Error>>;
type CheckRemoteCommitFn = fn(&str, &str, &str, &str) -> LibResult<CheckResult>;
type SelfUpdateFn = fn(&str, &str, &[&str]) -> LibResult<()>;

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

pub(crate) fn run_workspace_update_with(updater: SelfUpdateFn) -> Result<()> {
    updater(UPDATE_REPO_OWNER, UPDATE_REPO_NAME, &UPDATE_BINARIES)
        .map_err(|error| anyhow!("failed to update workspace: {error}"))
        .with_context(|| format!("install command: {}", workspace_install_command()))
}

pub fn run_workspace_update() -> Result<()> {
    run_workspace_update_with(self_update)
}
