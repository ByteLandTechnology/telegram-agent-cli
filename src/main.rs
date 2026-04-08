#[tokio::main]
async fn main() {
    if let Err(error) = telegram_cli::run().await {
        if let Some(rendered) = error.rendered() {
            eprintln!("{rendered}");
            std::process::exit(1);
        }

        let args: Vec<String> = std::env::args().collect();
        let format = telegram_cli::output::Format::detect_requested_format(&args[1..]);
        let command_path = error.command_path().map(str::to_owned).unwrap_or_else(|| {
            telegram_cli::output::guidance::canonical_command_path_from_args(&args[1..])
        });

        match format {
            telegram_cli::output::Format::Table => {
                let envelope = telegram_cli::output::contract::ResultEnvelope::error(
                    command_path,
                    "Command failed.",
                    error.code(),
                    error.to_string(),
                    telegram_cli::output::guidance::runtime_error_next_steps(
                        &telegram_cli::output::guidance::canonical_command_path_from_args(
                            &args[1..],
                        ),
                        &error.to_string(),
                    ),
                );
                match format.render(&envelope) {
                    Ok(rendered) => eprintln!("{rendered}"),
                    Err(render_error) => eprintln!("{render_error}"),
                }
            }
            _ => {
                let next_steps = if let Some(help_command) = error.help_command() {
                    vec![telegram_cli::output::contract::NextStep {
                        action: "inspect_help".into(),
                        command: help_command.to_string(),
                    }]
                } else {
                    telegram_cli::output::guidance::runtime_error_next_steps(
                        &telegram_cli::output::guidance::canonical_command_path_from_args(
                            &args[1..],
                        ),
                        &error.to_string(),
                    )
                };
                let envelope = telegram_cli::output::contract::ResultEnvelope::error(
                    command_path,
                    "Command failed.",
                    error.code(),
                    error.to_string(),
                    next_steps,
                );
                match format.render(&envelope) {
                    Ok(rendered) => eprintln!("{rendered}"),
                    Err(render_error) => eprintln!("{render_error}"),
                }
            }
        }
        std::process::exit(1);
    }
}
