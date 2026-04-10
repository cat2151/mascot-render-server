use std::error::Error;
use std::fmt;
use std::fs;
use std::process::Command;

use cat_self_update_lib::CheckResult;

use crate::workspace_update::{
    check_workspace_update_with, generate_update_script, run_workspace_update_with,
    workspace_install_command,
};

const FAKE_CHECK_ERROR_MESSAGE: &str = "network down";

#[test]
fn workspace_install_command_matches_readme_install_command() {
    assert_eq!(
        workspace_install_command(),
        "cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui"
    );
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

fn fake_update_success(owner: &str, repo: &str, bins: &[&str]) -> Result<(), Box<dyn Error>> {
    assert_eq!(owner, "cat2151");
    assert_eq!(repo, "mascot-render-server");
    assert_eq!(bins, ["mascot-render-server", "psd-viewer-tui"]);

    Ok(())
}

fn fake_update_failure(_owner: &str, _repo: &str, _bins: &[&str]) -> Result<(), Box<dyn Error>> {
    Err(Box::new(FakeCheckError))
}

fn assert_python_script_has_valid_syntax(script: &str) {
    let script_path = std::env::temp_dir().join(format!(
        "workspace_update_test_{}_{}.py",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&script_path, script).expect("should write generated script to a temp file");

    let python_candidates: &[(&str, &[&str])] = if cfg!(windows) {
        &[("python", &[]), ("py", &["-3"])]
    } else {
        &[("python3", &[]), ("python", &[])]
    };
    let compile_command =
        "import pathlib, sys; compile(pathlib.Path(sys.argv[1]).read_text(encoding='utf-8'), sys.argv[1], 'exec')";

    let mut output = None;
    for (program, args) in python_candidates {
        match Command::new(program)
            .args(*args)
            .arg("-c")
            .arg(compile_command)
            .arg(&script_path)
            .output()
        {
            Ok(candidate_output) => {
                output = Some((program, *args, candidate_output));
                break;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => panic!("failed to run {program}: {err}"),
        }
    }

    let (program, args, output) = match output {
        Some(output) => output,
        None => {
            fs::remove_file(&script_path).expect("failed to remove temporary test script");
            eprintln!("skipping Python syntax validation because no Python interpreter was found");
            return;
        }
    };
    let command = std::iter::once((*program).to_string())
        .chain(args.iter().map(|arg| (*arg).to_string()))
        .chain(["-c".to_string(), compile_command.to_string()])
        .collect::<Vec<_>>()
        .join(" ");

    fs::remove_file(&script_path).expect("failed to remove temporary test script");

    assert!(
        output.status.success(),
        "generated Python script has invalid syntax\ncommand: {command}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
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

#[test]
fn run_workspace_update_uses_workspace_repo_and_binary_arguments() {
    run_workspace_update_with(fake_update_success).expect("workspace update should succeed");
}

#[test]
fn generate_update_script_installs_all_workspace_binaries() {
    let script = generate_update_script(
        "cat2151",
        "mascot-render-server",
        &["mascot-render-server", "psd-viewer-tui"],
        1234,
    );

    assert!(script.contains(
        "INSTALL_PARTS = ['cargo', 'install', '--force', '--git', 'https://github.com/cat2151/mascot-render-server', 'mascot-render-server', 'psd-viewer-tui']"
    ));
    assert!(script.contains("    launch(['mascot-render-server'])"));
    assert!(script.contains("    launch(['psd-viewer-tui'])"));
}

#[test]
fn generate_update_script_keeps_top_level_python_lines_unindented() {
    let script = generate_update_script(
        "cat2151",
        "mascot-render-server",
        &["mascot-render-server", "psd-viewer-tui"],
        1234,
    );

    assert!(script.starts_with("import os\nimport shlex\nimport subprocess\n"));
    assert!(script.contains(
        "    launch(['psd-viewer-tui'])\nexcept subprocess.CalledProcessError as err:\n"
    ));
}

#[test]
fn generate_update_script_has_valid_python_syntax_for_multiple_binaries() {
    let script = generate_update_script(
        "cat2151",
        "mascot-render-server",
        &["mascot-render-server", "psd-viewer-tui"],
        1234,
    );

    assert_python_script_has_valid_syntax(&script);
}

#[test]
fn run_workspace_update_adds_context_to_errors() {
    let error = run_workspace_update_with(fake_update_failure).expect_err("should fail");

    assert_eq!(
        error.to_string(),
        format!("manual reinstall command: {}", workspace_install_command())
    );
    assert_eq!(
        error
            .source()
            .expect("context should retain the update failure")
            .to_string(),
        format!("failed to update workspace: {FAKE_CHECK_ERROR_MESSAGE}")
    );
}
