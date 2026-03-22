use std::ffi::OsString;
use std::path::PathBuf;

use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

use crate::cli::{help_text, parse_cli, CliAction};

#[test]
fn help_text_lists_local_data_defaults() {
    let help = help_text();

    assert!(help.contains("Commands:\n  update"));
    assert!(help.contains("Options:\n  --config <path>"));
    assert!(help.contains(&local_data_root().display().to_string()));
    assert!(help.contains(&workspace_cache_root().display().to_string()));
    assert!(help.contains(&mascot_config_path().display().to_string()));
}

#[test]
fn help_flag_returns_help_without_running_app() {
    let action = parse_cli([
        OsString::from("mascot-render-server"),
        OsString::from("--help"),
    ])
    .expect("help should parse");

    assert!(matches!(action, CliAction::PrintHelp(_)));
}

#[test]
fn config_flag_still_accepts_custom_path() {
    let action = parse_cli([
        OsString::from("mascot-render-server"),
        OsString::from("--config"),
        OsString::from("custom.toml"),
    ])
    .expect("config flag should parse");

    match action {
        CliAction::Run(path) => assert_eq!(path, PathBuf::from("custom.toml")),
        CliAction::Update | CliAction::PrintHelp(_) => panic!("expected run action"),
    }
}

#[test]
fn update_subcommand_returns_update_action() {
    let action = parse_cli([
        OsString::from("mascot-render-server"),
        OsString::from("update"),
    ])
    .expect("update should parse");

    assert!(matches!(action, CliAction::Update));
}
