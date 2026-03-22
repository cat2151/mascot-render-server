use std::ffi::OsString;

use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

use crate::cli::{help_text, parse_cli, CliAction};
use crate::tui_config::tui_config_path;

#[test]
fn help_text_lists_local_data_defaults() {
    let help = help_text();

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
