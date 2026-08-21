use std::ffi::OsString;

use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};
use mascot_render_control::server_performance_log_path;
use mascot_render_core::{local_data_root, mascot_config_path, workspace_cache_root};

#[derive(Debug)]
pub(crate) enum CliAction {
    Run,
    Check,
    PrintHelp(String),
}

#[derive(Debug, Parser)]
#[command(
    name = "mascot-render-status-tui",
    disable_help_subcommand = true,
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Check,
}

pub(crate) fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<CliAction> {
    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Some(Commands::Check) => Ok(CliAction::Check),
            None => Ok(CliAction::Run),
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
    let mascot_config = mascot_config_path();
    let performance_log = server_performance_log_path();

    format!(
        "\
Usage:
  mascot-render-status-tui
  mascot-render-status-tui check

Commands:
  check           Compare the embedded commit hash with the remote main branch.

Options:
  -h, --help      Show this help.

Default local data directory:
  {}

Default paths:
  assets           {}
  cache            {}
  server config    {}
  performance log  {}
",
        data_root.display(),
        assets_root.display(),
        cache_root.display(),
        mascot_config.display(),
        performance_log.display()
    )
}
