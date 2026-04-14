use std::ffi::OsString;

use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

use crate::cli::{help_text, parse_cli, CliAction};
use crate::tui_config::tui_config_path;

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

    assert!(help.contains("Commands:\n  update"));
    assert!(help.contains("Options:\n  -h, --help"));
    assert!(help.contains(&local_data_root().display().to_string()));
    assert!(help.contains(&workspace_cache_root().display().to_string()));
    assert!(help.contains(&tui_config_path().display().to_string()));
    assert!(help.contains(&mascot_config_path().display().to_string()));
}

#[test]
fn help_flag_returns_help_without_starting_terminal() {
    let action =
        parse_cli([OsString::from("psd-viewer-tui"), OsString::from("--help")]).expect("help");

    assert!(matches!(action, CliAction::PrintHelp(_)));
}

#[test]
fn unsupported_flag_returns_error() {
    let error = parse_cli([
        OsString::from("psd-viewer-tui"),
        OsString::from("--unknown"),
    ])
    .expect_err("unknown flag should fail");

    assert!(error.to_string().contains("--unknown"));
}

#[test]
fn update_subcommand_returns_update_action() {
    let action =
        parse_cli([OsString::from("psd-viewer-tui"), OsString::from("update")]).expect("update");

    assert!(matches!(action, CliAction::Update));
}

#[test]
fn check_subcommand_returns_check_action() {
    let action =
        parse_cli([OsString::from("psd-viewer-tui"), OsString::from("check")]).expect("check");

    assert!(matches!(action, CliAction::Check));
}

#[test]
fn update_subcommand_help_returns_help_text() {
    let action = parse_cli([
        OsString::from("psd-viewer-tui"),
        OsString::from("update"),
        OsString::from("--help"),
    ])
    .expect("update help");

    assert!(matches!(action, CliAction::PrintHelp(_)));
}
