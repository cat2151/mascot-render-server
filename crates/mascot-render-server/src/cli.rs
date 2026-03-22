use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use mascot_render_core::{
    local_data_root, mascot_config_path, parse_mascot_config_path, workspace_cache_root,
};

#[derive(Debug)]
pub(crate) enum CliAction {
    Run(PathBuf),
    PrintHelp(String),
}

pub(crate) fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<CliAction> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .iter()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        return Ok(CliAction::PrintHelp(help_text()));
    }

    parse_mascot_config_path(args).map(CliAction::Run)
}

pub(crate) fn help_text() -> String {
    let data_root = local_data_root();
    let assets_root = data_root.join("assets");
    let cache_root = workspace_cache_root();
    let config_path = mascot_config_path();

    format!(
        "\
Usage:
  mascot-render-server [--config <path>]

Options:
  --config <path>  Use a custom mascot static config TOML.
  -h, --help       Show this help.

Default local data directory:
  {}

Default paths:
  assets  {}
  cache   {}
  config  {}
",
        data_root.display(),
        assets_root.display(),
        cache_root.display(),
        config_path.display()
    )
}
