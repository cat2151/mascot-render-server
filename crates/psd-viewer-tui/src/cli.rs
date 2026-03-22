use std::ffi::OsString;

use anyhow::{bail, Result};
use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

use crate::tui_config::tui_config_path;

#[derive(Debug)]
pub(crate) enum CliAction {
    Run,
    Update,
    PrintHelp(String),
}

pub(crate) fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<CliAction> {
    let mut args = args.into_iter();
    let _program = args.next();

    match args.next() {
        None => return Ok(CliAction::Run),
        Some(arg) if arg == "--help" || arg == "-h" => {
            return Ok(CliAction::PrintHelp(help_text()));
        }
        Some(arg) if arg == "update" => {
            if let Some(next) = args.next() {
                if next == "--help" || next == "-h" {
                    return Ok(CliAction::PrintHelp(help_text()));
                }
                bail!(
                    "unsupported argument '{}' after 'update'; run with --help for usage",
                    next.to_string_lossy()
                );
            }
            return Ok(CliAction::Update);
        }
        Some(arg) if arg.to_string_lossy().starts_with('-') => {
            bail!(
                "unsupported argument '{}'; run with --help for usage",
                arg.to_string_lossy()
            );
        }
        Some(arg) => {
            bail!(
                "unsupported positional argument '{}'; run with --help for usage",
                arg.to_string_lossy()
            );
        }
    }
}

pub(crate) fn help_text() -> String {
    let data_root = local_data_root();
    let assets_root = data_root.join("assets");
    let cache_root = workspace_cache_root();
    let tui_config = tui_config_path();
    let mascot_config = mascot_config_path();

    format!(
        "\
Usage:
  psd-viewer-tui
  psd-viewer-tui update

Commands:
  update          Stop running binaries and reinstall both binaries.

Options:
  -h, --help      Show this help.

Default local data directory:
  {}

Default paths:
  assets         {}
  cache          {}
  tui config     {}
  server config  {}
",
        data_root.display(),
        assets_root.display(),
        cache_root.display(),
        tui_config.display(),
        mascot_config.display()
    )
}
