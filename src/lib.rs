pub mod app;
pub mod automation;
pub mod cli;
pub mod commands;
pub mod config;
pub mod errors;
pub mod output;
pub mod storage;
pub mod telegram;

use clap::{error::ErrorKind, Parser};

pub async fn run() -> errors::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cli = match cli::Cli::try_parse_from(args.clone()) {
        Ok(cli) => cli,
        Err(error) => match error.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{error}");
                return Ok(());
            }
            ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                let help_path = output::guidance::closest_help_path_from_args(&args[1..]);
                if !output::guidance::is_leaf_help_path(&help_path) {
                    output::guidance::print_human_help_for_path(&help_path)?;
                    return Ok(());
                }
                return Err(output::guidance::cli_usage_error(&args[1..], error));
            }
            _ => return Err(output::guidance::cli_usage_error(&args[1..], error)),
        },
    };
    app::run(cli).await
}
