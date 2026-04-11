use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};
use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

#[derive(Debug)]
pub(crate) enum CliAction {
    Run(PathBuf),
    Update,
    Check,
    PrintHelp(String),
}

#[derive(Debug, Parser)]
#[command(
    name = "mascot-render-server",
    disable_help_subcommand = true,
    disable_version_flag = true,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[arg(long, value_name = "path")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Update,
    Check,
}

pub(crate) fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<CliAction> {
    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Some(Commands::Update) => Ok(CliAction::Update),
            Some(Commands::Check) => Ok(CliAction::Check),
            None => Ok(CliAction::Run(
                cli.config.unwrap_or_else(mascot_config_path),
            )),
        },
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            Ok(CliAction::PrintHelp(help_text()))
        }
        Err(error) => Err(error.into()),
    }
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
  mascot-render-server update
  mascot-render-server check

Commands:
  update           Stop running binaries and reinstall workspace binaries.
  check            Compare the embedded commit hash with the remote main branch.

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
