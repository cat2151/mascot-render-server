use std::ffi::OsString;

use mascot_render_control::server_performance_log_path;
use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

use crate::cli::{help_text, parse_cli, CliAction};

#[test]
fn local_data_root_is_redirected_to_temp_directory_for_tests() {
    assert!(
        local_data_root().starts_with(std::env::temp_dir()),
        "test local data root should live under temp dir: {}",
        local_data_root().display()
    );
}

#[test]
fn help_text_lists_local_data_defaults() {
    let help = help_text();

    assert!(help.contains("Commands:\n  check"));
    assert!(help.contains("Options:\n  -h, --help"));
    assert!(help.contains(&local_data_root().display().to_string()));
    assert!(help.contains(&workspace_cache_root().display().to_string()));
    assert!(help.contains(&mascot_config_path().display().to_string()));
    assert!(help.contains(&server_performance_log_path().display().to_string()));
}

#[test]
fn help_flag_returns_help_without_starting_terminal() {
    let action = parse_cli([
        OsString::from("mascot-render-status-tui"),
        OsString::from("--help"),
    ])
    .expect("help");

    assert!(matches!(action, CliAction::PrintHelp(_)));
}

#[test]
fn no_arguments_returns_run_action() {
    let action = parse_cli([OsString::from("mascot-render-status-tui")]).expect("run");

    assert!(matches!(action, CliAction::Run));
}

#[test]
fn unsupported_flag_returns_error() {
    let error = parse_cli([
        OsString::from("mascot-render-status-tui"),
        OsString::from("--unknown"),
    ])
    .expect_err("unknown flag should fail");

    assert!(error.to_string().contains("--unknown"));
}

#[test]
fn check_subcommand_returns_check_action() {
    let action = parse_cli([
        OsString::from("mascot-render-status-tui"),
        OsString::from("check"),
    ])
    .expect("check");

    assert!(matches!(action, CliAction::Check));
}

#[test]
fn check_subcommand_help_returns_help_text() {
    let action = parse_cli([
        OsString::from("mascot-render-status-tui"),
        OsString::from("check"),
        OsString::from("--help"),
    ])
    .expect("check help");

    assert!(matches!(action, CliAction::PrintHelp(_)));
}
