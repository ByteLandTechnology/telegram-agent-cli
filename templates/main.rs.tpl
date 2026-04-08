//! Baseline CLI entrypoint for the generated package layout. Optional
//! capabilities may extend the package with package-local support files, but
//! repository-owned CI and release automation stay outside generated outputs.

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use {{SKILL_NAME_SNAKE}}::context::{
    build_context_state, inspect_context, parse_selectors, persist_active_context,
    resolve_effective_context, resolve_runtime_locations, InvocationContextOverrides,
    RuntimeOverrides,
};
use {{SKILL_NAME_SNAKE}}::help::{plain_text_help, structured_help};
use {{SKILL_NAME_SNAKE}}::{run, serialize_value, write_structured_error, Format, StructuredError};

#[derive(Debug)]
enum AppExit {
    Usage,
    Failure(anyhow::Error),
}

impl From<anyhow::Error> for AppExit {
    fn from(error: anyhow::Error) -> Self {
        Self::Failure(error)
    }
}

/// {{DESCRIPTION}}
#[derive(Parser, Debug)]
#[command(
    name = "{{SKILL_NAME}}",
    version,
    about = "{{DESCRIPTION}}",
    disable_help_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    /// Output format
    #[arg(long, short, value_enum, global = true, default_value_t = OutputFormat::Yaml)]
    format: OutputFormat,

    /// Render plain-text help for the selected command path
    #[arg(long, short = 'h', global = true, action = ArgAction::SetTrue)]
    help: bool,

    /// Override the default configuration directory
    #[arg(long, global = true)]
    config_dir: Option<PathBuf>,

    /// Override the default durable data directory
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Override the runtime state directory
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    /// Override the cache directory
    #[arg(long, global = true)]
    cache_dir: Option<PathBuf>,

    /// Override the optional log directory
    #[arg(long, global = true)]
    log_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Yaml,
    Json,
    Toml,
}

impl From<OutputFormat> for Format {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Yaml => Format::Yaml,
            OutputFormat::Json => Format::Json,
            OutputFormat::Toml => Format::Toml,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Return machine-readable help for a command path
    Help(HelpCommand),
    /// Execute the generated leaf command
    Run(RunCommand),
    /// Inspect runtime directory defaults and overrides
    Paths(PathsCommand),
    /// Inspect or persist the Active Context
    Context(ContextCommand),
}

#[derive(Debug, Args)]
struct HelpCommand {
    /// Command path to inspect
    #[arg(value_name = "COMMAND_PATH")]
    path: Vec<String>,
}

#[derive(Debug, Args)]
struct RunCommand {
    /// Required input for the generated leaf command
    input: Option<String>,

    /// Explicit per-invocation context selector override
    #[arg(long = "selector", value_name = "KEY=VALUE")]
    selectors: Vec<String>,

    /// Explicit current-directory ambient cue
    #[arg(long = "cwd")]
    current_directory: Option<PathBuf>,

    /// Include the optional log directory in the resolved runtime paths
    #[arg(long)]
    log_enabled: bool,
}

#[derive(Debug, Args)]
struct PathsCommand {
    /// Include the optional log directory in the resolved runtime paths
    #[arg(long)]
    log_enabled: bool,
}

#[derive(Debug, Args)]
struct ContextCommand {
    #[command(subcommand)]
    command: Option<ContextSubcommand>,
}

#[derive(Debug, Subcommand)]
enum ContextSubcommand {
    /// Display the current persisted and effective context
    Show,
    /// Persist selectors and ambient cues as the Active Context
    Use(ContextUseCommand),
}

#[derive(Debug, Args)]
struct ContextUseCommand {
    /// Optional label for the persisted context
    #[arg(long)]
    name: Option<String>,

    /// Selector to persist in the Active Context
    #[arg(long = "selector", value_name = "KEY=VALUE")]
    selectors: Vec<String>,

    /// Ambient current-directory cue to persist
    #[arg(long = "cwd")]
    current_directory: Option<PathBuf>,
}

#[derive(Debug, serde::Serialize)]
struct RunResponse {
    status: String,
    message: String,
    input: String,
    effective_context: std::collections::BTreeMap<String, String>,
}

fn main() {
    let exit_code = match run_cli() {
        Ok(()) => 0,
        Err(AppExit::Usage) => 2,
        Err(AppExit::Failure(error)) => {
            eprintln!("error: {error:#}");
            1
        }
    };

    std::process::exit(exit_code);
}

fn run_cli() -> std::result::Result<(), AppExit> {
    let raw_args: Vec<String> = std::env::args().collect();
    let detected_format = detect_requested_format(&raw_args);

    let cli = match Cli::try_parse_from(&raw_args) {
        Ok(cli) => cli,
        Err(error) => return handle_parse_error(error, detected_format),
    };

    let format: Format = cli.format.into();

    if cli.help {
        return render_plain_text_help_for_cli(&cli);
    }

    let runtime_overrides = cli_runtime_overrides(&cli);

    match cli.command {
        None => render_plain_text_help_for_path(&[]),
        Some(Command::Help(command)) => render_structured_help(&command.path, format),
        Some(Command::Run(command)) => execute_run(runtime_overrides, command, format),
        Some(Command::Paths(command)) => execute_paths(runtime_overrides, command, format),
        Some(Command::Context(command)) => execute_context(runtime_overrides, command, format),
    }
}

fn handle_parse_error(error: clap::Error, format: Format) -> std::result::Result<(), AppExit> {
    if error.kind() == clap::error::ErrorKind::DisplayVersion {
        error.print().map_err(|err| AppExit::Failure(err.into()))?;
        return Ok(());
    }

    let structured_error =
        StructuredError::new("usage.parse_error", error.to_string(), "help_usage", format);
    let mut stderr = std::io::stderr().lock();
    write_structured_error(&mut stderr, &structured_error, format).map_err(AppExit::from)?;
    Err(AppExit::Usage)
}

fn cli_runtime_overrides(cli: &Cli) -> RuntimeOverrides {
    RuntimeOverrides {
        config_dir: cli.config_dir.clone(),
        data_dir: cli.data_dir.clone(),
        state_dir: cli.state_dir.clone(),
        cache_dir: cli.cache_dir.clone(),
        log_dir: cli.log_dir.clone(),
    }
}

fn detect_requested_format(args: &[String]) -> Format {
    let mut args = args.iter().peekable();
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--format=") {
            return parse_format_token(value).unwrap_or(Format::Yaml);
        }
        if arg == "--format" || arg == "-f" {
            if let Some(value) = args.peek() {
                return parse_format_token(value).unwrap_or(Format::Yaml);
            }
        }
    }
    Format::Yaml
}

fn parse_format_token(token: &str) -> Option<Format> {
    match token {
        "yaml" => Some(Format::Yaml),
        "json" => Some(Format::Json),
        "toml" => Some(Format::Toml),
        _ => None,
    }
}

fn render_plain_text_help_for_cli(cli: &Cli) -> std::result::Result<(), AppExit> {
    let path = match &cli.command {
        None => Vec::new(),
        Some(Command::Help(_)) => vec!["help".to_string()],
        Some(Command::Run(_)) => vec!["run".to_string()],
        Some(Command::Paths(_)) => vec!["paths".to_string()],
        Some(Command::Context(ContextCommand { command: None })) => vec!["context".to_string()],
        Some(Command::Context(ContextCommand {
            command: Some(ContextSubcommand::Show),
        })) => vec!["context".to_string(), "show".to_string()],
        Some(Command::Context(ContextCommand {
            command: Some(ContextSubcommand::Use(_)),
        })) => vec!["context".to_string(), "use".to_string()],
    };

    render_plain_text_help_for_path(&path)
}

fn render_plain_text_help_for_path(path: &[String]) -> std::result::Result<(), AppExit> {
    let Some(help_text) = plain_text_help(path) else {
        let mut stderr = std::io::stderr().lock();
        let error = StructuredError::new(
            "help.unknown_path",
            format!("unknown help path '{}'", path.join(" ")),
            "help_usage",
            Format::Yaml,
        );
        write_structured_error(&mut stderr, &error, Format::Yaml).map_err(AppExit::from)?;
        return Err(AppExit::Usage);
    };

    println!("{help_text}");
    Ok(())
}

fn render_structured_help(path: &[String], format: Format) -> std::result::Result<(), AppExit> {
    let Some(help_document) = structured_help(path) else {
        let mut stderr = std::io::stderr().lock();
        let error = StructuredError::new(
            "help.unknown_path",
            format!("unknown help path '{}'", path.join(" ")),
            "help_usage",
            format,
        );
        write_structured_error(&mut stderr, &error, format).map_err(AppExit::from)?;
        return Err(AppExit::Usage);
    };

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    serialize_value(&mut stdout, &help_document, format).map_err(AppExit::from)?;
    Ok(())
}

fn execute_paths(
    overrides: RuntimeOverrides,
    command: PathsCommand,
    format: Format,
) -> std::result::Result<(), AppExit> {
    let runtime =
        resolve_runtime_locations(&overrides, command.log_enabled).map_err(AppExit::from)?;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    serialize_value(&mut stdout, &runtime.summary(), format).map_err(AppExit::from)?;
    Ok(())
}

fn execute_context(
    overrides: RuntimeOverrides,
    command: ContextCommand,
    format: Format,
) -> std::result::Result<(), AppExit> {
    match command.command {
        None => render_plain_text_help_for_path(&["context".to_string()]),
        Some(ContextSubcommand::Show) => {
            let runtime = resolve_runtime_locations(&overrides, false).map_err(AppExit::from)?;
            let inspection = inspect_context(&runtime, &InvocationContextOverrides::default())
                .map_err(AppExit::from)?;
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            serialize_value(&mut stdout, &inspection, format).map_err(AppExit::from)?;
            Ok(())
        }
        Some(ContextSubcommand::Use(command)) => {
            let selectors = parse_selectors(&command.selectors).map_err(AppExit::from)?;
            let current_directory = command.current_directory;
            if selectors.is_empty() && current_directory.is_none() && command.name.is_none() {
                let error = StructuredError::new(
                    "context.missing_values",
                    "provide at least one --selector, --cwd, or --name when persisting an Active Context",
                    "runtime_state",
                    format,
                );
                let mut stderr = std::io::stderr().lock();
                write_structured_error(&mut stderr, &error, format).map_err(AppExit::from)?;
                return Err(AppExit::Usage);
            }

            let runtime = resolve_runtime_locations(&overrides, false).map_err(AppExit::from)?;
            let state = build_context_state(command.name, selectors, current_directory);
            let persisted = persist_active_context(&runtime, &state).map_err(AppExit::from)?;
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            serialize_value(&mut stdout, &persisted, format).map_err(AppExit::from)?;
            Ok(())
        }
    }
}

fn execute_run(
    overrides: RuntimeOverrides,
    command: RunCommand,
    format: Format,
) -> std::result::Result<(), AppExit> {
    let runtime =
        resolve_runtime_locations(&overrides, command.log_enabled).map_err(AppExit::from)?;
    let selectors = parse_selectors(&command.selectors).map_err(AppExit::from)?;
    let invocation_overrides = InvocationContextOverrides {
        selectors,
        current_directory: command.current_directory,
    };
    let persisted_context =
        {{SKILL_NAME_SNAKE}}::context::load_active_context(&runtime).map_err(AppExit::from)?;
    let effective_context =
        resolve_effective_context(persisted_context.as_ref(), &invocation_overrides);

    let Some(input) = command.input else {
        let error = StructuredError::new(
            "run.missing_input",
            "the run command requires <INPUT>; use --help for plain-text help",
            "leaf_validation",
            format,
        )
        .with_detail("command", "run");
        let mut stderr = std::io::stderr().lock();
        write_structured_error(&mut stderr, &error, format).map_err(AppExit::from)?;
        return Err(AppExit::Usage);
    };

    let response = run(&input, effective_context.effective_values.clone());
    let output = RunResponse {
        status: response.status,
        message: response.message,
        input: response.input,
        effective_context: response.effective_context,
    };

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    serialize_value(&mut stdout, &output, format).map_err(AppExit::from)?;
    Ok(())
}
