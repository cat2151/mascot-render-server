use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use cat_self_update_lib::{check_remote_commit, CheckResult};

const UPDATE_REPO_OWNER: &str = "cat2151";
const UPDATE_REPO_NAME: &str = "mascot-render-server";
const UPDATE_BRANCH: &str = "main";
const UPDATE_GIT_URL: &str = "https://github.com/cat2151/mascot-render-server";
const UPDATE_TARGETS: [&str; 2] = ["mascot-render-server", "psd-viewer-tui"];

pub fn workspace_install_command() -> String {
    format!(
        "cargo install --force --git {UPDATE_GIT_URL} {}",
        UPDATE_TARGETS.join(" ")
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
    updater(UPDATE_REPO_OWNER, UPDATE_REPO_NAME, &UPDATE_TARGETS)
        .map_err(|error| anyhow!("failed to update workspace: {error}"))
        .with_context(|| format!("manual reinstall command: {}", workspace_install_command()))
}

pub fn run_workspace_update() -> Result<()> {
    run_workspace_update_with(self_update_workspace)
}

fn self_update_workspace(owner: &str, repo: &str, bins: &[&str]) -> LibResult<()> {
    let py_content = generate_update_script(owner, repo, bins, std::process::id());
    let py_path = unique_tmp_path();

    fs::write(&py_path, py_content)?;
    spawn_python(&py_path)?;

    Ok(())
}

fn escape_py_single_quoted(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

fn python_list_literal<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let quoted_parts = parts
        .into_iter()
        .map(escape_py_single_quoted)
        .map(|part| format!("'{part}'"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{quoted_parts}]")
}

pub(crate) fn generate_update_script(
    owner: &str,
    repo: &str,
    bins: &[&str],
    parent_pid: u32,
) -> String {
    let repo_url = format!("https://github.com/{owner}/{repo}");
    let install_parts = if bins.is_empty() {
        python_list_literal(["cargo", "install", "--force", "--git", repo_url.as_str()])
    } else {
        python_list_literal(
            ["cargo", "install", "--force", "--git", repo_url.as_str()]
                .into_iter()
                .chain(bins.iter().copied()),
        )
    };

    let launch_stmts = if bins.is_empty() {
        let repo_escaped = escape_py_single_quoted(repo);
        format!("    launch(['{repo_escaped}'])\n")
    } else {
        bins.iter()
            .map(|bin| {
                let bin_escaped = escape_py_single_quoted(bin);
                format!("    launch(['{bin_escaped}'])\n")
            })
            .collect::<String>()
    };

    format!(
        concat!(
            "import os\n",
            "import shlex\n",
            "import subprocess\n",
            "import sys\n",
            "import traceback\n",
            "\n",
            "PARENT_PID = {parent_pid}\n",
            "INSTALL_PARTS = {install_parts}\n",
            "\n",
            "def log(message):\n",
            "    print(message, flush=True)\n",
            "\n",
            "def format_command(parts):\n",
            "    if sys.platform == 'win32':\n",
            "        return subprocess.list2cmdline(parts)\n",
            "    return shlex.join(parts)\n",
            "\n",
            "def wait_for_parent_exit():\n",
            "    if sys.platform != 'win32':\n",
            "        return\n",
            "\n",
            "    import ctypes\n",
            "\n",
            "    synchronize = 0x00100000\n",
            "    infinite = 0xFFFFFFFF\n",
            "    kernel32 = ctypes.windll.kernel32\n",
            "    handle = kernel32.OpenProcess(synchronize, False, PARENT_PID)\n",
            "    if not handle:\n",
            "        return\n",
            "\n",
            "    try:\n",
            "        kernel32.WaitForSingleObject(handle, infinite)\n",
            "    finally:\n",
            "        kernel32.CloseHandle(handle)\n",
            "\n",
            "def launch(parts):\n",
            "    log(f\"起動しています: {{format_command(parts)}}\")\n",
            "    subprocess.Popen(parts)\n",
            "\n",
            "def wait_for_user_acknowledgement():\n",
            "    if sys.platform != 'win32':\n",
            "        return\n",
            "\n",
            "    log(\"Enterキーを押すと閉じます\")\n",
            "    try:\n",
            "        input()\n",
            "    except EOFError:\n",
            "        pass\n",
            "\n",
            "try:\n",
            "    log(\"現在のプロセスの終了を待っています\")\n",
            "    wait_for_parent_exit()\n",
            "    log(\"cargo installを起動しています\")\n",
            "    log(f\"$ {{format_command(INSTALL_PARTS)}}\")\n",
            "    subprocess.run(INSTALL_PARTS, check=True)\n",
            "    log(\"cargo install が完了しました\")\n",
            "{launch_stmts}",
            "except subprocess.CalledProcessError as err:\n",
            "    log(f\"cargo install が失敗しました。終了コード: {{err.returncode}}\")\n",
            "    wait_for_user_acknowledgement()\n",
            "    sys.exit(err.returncode)\n",
            "except Exception as err:\n",
            "    log(f\"更新処理に失敗しました: {{err}}\")\n",
            "    traceback.print_exc()\n",
            "    wait_for_user_acknowledgement()\n",
            "    sys.exit(1)\n",
            "finally:\n",
            "    try:\n",
            "        os.remove(__file__)\n",
            "    except OSError:\n",
            "        pass\n"
        ),
        parent_pid = parent_pid,
        install_parts = install_parts,
        launch_stmts = launch_stmts
    )
}

fn unique_tmp_path() -> PathBuf {
    let pid = std::process::id();
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("mascot_render_update_{pid}_{timestamp_nanos}.py");

    std::env::temp_dir().join(filename)
}

fn spawn_python(py_path: &Path) -> LibResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new("python")
            .arg(py_path)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()?;

        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let mut candidates = Vec::new();
        if let Some(python) = std::env::var_os("PYTHON") {
            candidates.push(python);
        }
        candidates.push("python3".into());
        candidates.push("python".into());
        let tried = candidates
            .iter()
            .map(|candidate| candidate.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(", ");

        let mut last_not_found = None;
        for candidate in candidates {
            match Command::new(&candidate).arg(py_path).spawn() {
                Ok(_) => return Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    last_not_found = Some(err);
                }
                Err(err) => {
                    return Err(std::io::Error::new(
                        err.kind(),
                        format!(
                            "failed to launch Python interpreter {}: {err}",
                            candidate.to_string_lossy()
                        ),
                    )
                    .into());
                }
            }
        }

        Err(last_not_found
            .unwrap_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no Python interpreter found; tried {tried}"),
                )
            })
            .into())
    }
}
