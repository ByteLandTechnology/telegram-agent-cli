use crate::errors::{Result, TelegramCliError};
use crate::output::contract::{
    GuidanceKind, GuidanceSurface, HelpArgumentEntry, HelpCommandEntry, HelpDocument,
    HelpOptionEntry, HelpRuntimeState, NextStep,
};
use crate::output::Format;
use clap::error::ErrorKind;
use clap::CommandFactory;

pub const CANONICAL_TERMS: &[(&str, &str)] = &[
    (
        "account",
        "A configured Telegram identity that telegram-agent-cli can act as.",
    ),
    (
        "peer",
        "A resolved Telegram target such as a user, bot, group, or channel.",
    ),
    (
        "chat",
        "A Telegram conversation target referenced by alias, username, or ID.",
    ),
    (
        "alias",
        "A local shortcut name that maps to a resolved peer.",
    ),
    (
        "scenario",
        "A stored automation script whose run emits exportable events.",
    ),
    (
        "run",
        "One execution of a scenario file recorded in local storage.",
    ),
    (
        "output format",
        "The requested projection of a normalized result or guidance contract.",
    ),
];

pub fn print_help_for_path(path: &[String], args: &[String]) -> Result<()> {
    println!("{}", render_help_for_path(path, args)?);
    Ok(())
}

pub fn print_human_help_for_path(path: &[String]) -> Result<()> {
    println!("{}", render_help_for_path_with_format(path, Format::Table)?);
    Ok(())
}

pub fn render_help_for_path(path: &[String], args: &[String]) -> Result<String> {
    let format = Format::detect_requested_format_or(args, Format::Yaml);
    render_help_for_path_with_format(path, format)
}

pub fn render_help_document_for_path(path: &[String]) -> Result<HelpDocument> {
    build_help_document(path)
}

pub fn render_help_for_path_with_format(path: &[String], format: Format) -> Result<String> {
    let document = build_help_document(path)?;
    match format {
        Format::Table => render_table_help_for_path(path),
        _ => format.render(&document),
    }
}

fn render_table_help_for_path(path: &[String]) -> Result<String> {
    let surface = help_surface_for_path(path).ok_or_else(|| {
        TelegramCliError::Message(format!("unsupported help path: {}", path.join(" ")))
    })?;
    let mut command = clap_command_for_help_path(path)?;
    let mut rendered = command
        .render_long_help()
        .to_string()
        .trim_end()
        .to_string();
    append_help_footer(&mut rendered, &surface);
    Ok(rendered)
}

pub fn cli_usage_error(args: &[String], error: clap::Error) -> TelegramCliError {
    let help_path = closest_help_path_from_args(args);
    let command_path = canonical_command_path_from_args(args);
    let help_command = help_command_for_path(&help_path);
    let message = cli_usage_message(error.kind(), &error, &help_path);
    let rendered = cli_usage_rendered(&help_path, args, &message, &error);

    TelegramCliError::CliUsage {
        command_path,
        help_command,
        message,
        rendered,
    }
}

pub fn maybe_help_path(args: &[String]) -> Option<Vec<String>> {
    if args.is_empty() {
        return None;
    }

    if let Some(path) = explicit_help_subcommand_path(args) {
        return Some(path);
    }

    let mut path = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Some(path),
            "--format" => {
                let _ = iter.next();
            }
            "--json" => {}
            other if other.starts_with("--format=") => {}
            other if other.starts_with('-') => {}
            other => path.push(other.to_string()),
        }
    }

    None
}

pub fn canonical_command_path_from_args(args: &[String]) -> String {
    if let Some(path) = explicit_help_subcommand_path(args) {
        return render_command_path(&path);
    }

    let mut iter = args.iter().peekable();
    let mut command = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                let _ = iter.next();
            }
            other if other.starts_with('-') => {}
            other => {
                command = Some(other);
                break;
            }
        }
    }

    match command {
        None => "telegram-agent-cli".into(),
        Some(command @ ("account" | "alias" | "bot" | "list" | "message" | "peer")) => {
            let subcommand = iter.find(|value| !value.starts_with('-'));
            match subcommand {
                Some(subcommand) => format!("telegram-agent-cli {command} {subcommand}"),
                None => format!("telegram-agent-cli {command}"),
            }
        }
        Some(command) => format!("telegram-agent-cli {command}"),
    }
}

fn explicit_help_subcommand_path(args: &[String]) -> Option<Vec<String>> {
    let mut iter = args.iter().peekable();
    let mut positionals = Vec::new();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                let _ = iter.next();
            }
            "--json" => {}
            other if other.starts_with("--format=") => {}
            other if other.starts_with('-') => {}
            other => positionals.push(other.to_string()),
        }
    }

    match positionals.as_slice() {
        [help, rest @ ..] if help == "help" => Some(rest.to_vec()),
        _ => None,
    }
}

fn render_command_path(path: &[String]) -> String {
    if path.is_empty() {
        "telegram-agent-cli".into()
    } else {
        format!("telegram-agent-cli {}", path.join(" "))
    }
}

pub fn closest_help_path_from_args(args: &[String]) -> Vec<String> {
    let mut best = Vec::new();
    let mut current = Vec::new();

    for positional in positional_args(args) {
        current.push(positional);
        if help_surface_for_path(&current).is_some() {
            best = current.clone();
            if crate::cli::leaf_help_metadata(&render_command_path(&current)).is_some() {
                break;
            }
        } else {
            break;
        }
    }

    best
}

pub fn is_leaf_help_path(path: &[String]) -> bool {
    matches!(
        help_surface_for_path(path).map(|surface| surface.guidance_kind),
        Some(GuidanceKind::LeafHelp)
    )
}

fn clap_command_for_help_path(path: &[String]) -> Result<clap::Command> {
    let command = crate::cli::Cli::command();
    if path.is_empty() {
        return Ok(command);
    }

    let mut resolved = command
        .find_subcommand(path[0].as_str())
        .cloned()
        .ok_or_else(|| {
            TelegramCliError::Message(format!("unsupported help path: {}", path.join(" ")))
        })?;

    for segment in &path[1..] {
        resolved = resolved
            .find_subcommand(segment.as_str())
            .cloned()
            .ok_or_else(|| {
                TelegramCliError::Message(format!("unsupported help path: {}", path.join(" ")))
            })?;
    }

    Ok(resolved.bin_name(render_command_path(path)))
}

fn build_help_document(path: &[String]) -> Result<HelpDocument> {
    let surface = help_surface_for_path(path).ok_or_else(|| {
        TelegramCliError::Message(format!("unsupported help path: {}", path.join(" ")))
    })?;
    let mut command = clap_command_for_help_path(path)?;
    let usage = command.render_usage().to_string().trim().to_string();

    Ok(HelpDocument {
        command: surface
            .command_path
            .clone()
            .unwrap_or_else(|| render_command_path(path)),
        summary: surface.summary,
        usage,
        commands: command
            .get_subcommands()
            .filter(|entry| !entry.is_hide_set())
            .map(help_command_entry)
            .collect(),
        arguments: command
            .get_positionals()
            .filter(|arg| !arg.is_hide_set())
            .map(help_argument_entry)
            .collect(),
        options: command
            .get_opts()
            .filter(|arg| !arg.is_hide_set())
            .map(help_option_entry)
            .collect(),
        before_you_run_it: surface.prerequisites,
        examples: surface.examples,
        see_also: surface.related_commands,
        try_next: surface.next_steps,
        default_behavior: default_behavior_for_path(path),
        supported_output_formats: supported_output_formats(),
        runtime_state: runtime_state_for_path(path),
    })
}

fn append_help_footer(rendered: &mut String, surface: &GuidanceSurface) {
    match surface.guidance_kind {
        GuidanceKind::TopLevelHelp => {
            append_help_section(rendered, "Start with:", &surface.next_steps);
        }
        GuidanceKind::CommandGroupHelp => {
            append_help_section(rendered, "See also:", &surface.related_commands);
            append_help_section(rendered, "Try next:", &surface.next_steps);
        }
        GuidanceKind::LeafHelp => {
            append_help_section(rendered, "Before you run it:", &surface.prerequisites);
            append_help_section(rendered, "Examples:", &surface.examples);
            append_help_section(rendered, "See also:", &surface.related_commands);
            append_help_section(rendered, "Try next:", &surface.next_steps);
        }
        GuidanceKind::ReplHelp => {
            append_help_section(rendered, "Before you run it:", &surface.prerequisites);
            append_help_section(rendered, "Interactive commands:", &surface.actions);
            append_help_section(rendered, "Examples:", &surface.examples);
            append_help_section(rendered, "See also:", &surface.related_commands);
            append_help_section(rendered, "Try next:", &surface.next_steps);
        }
        GuidanceKind::RuntimeSuccess | GuidanceKind::RuntimeError => {}
    }
}

fn help_command_for_path(path: &[String]) -> String {
    if path.is_empty() {
        "telegram-agent-cli help".into()
    } else {
        format!("telegram-agent-cli help {}", path.join(" "))
    }
}

fn cli_usage_message(kind: ErrorKind, error: &clap::Error, help_path: &[String]) -> String {
    match kind {
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            if help_path.is_empty() {
                "Choose a command to continue.".into()
            } else {
                "Choose a subcommand to continue.".into()
            }
        }
        _ => clap_error_message(error).unwrap_or_else(|| "Command usage is invalid.".into()),
    }
}

fn cli_usage_rendered(
    help_path: &[String],
    args: &[String],
    message: &str,
    error: &clap::Error,
) -> String {
    let format = Format::detect_requested_format(args);
    if is_leaf_help_path(help_path) && format != Format::Table {
        let envelope = crate::output::contract::ResultEnvelope::error(
            canonical_command_path_from_args(args),
            "Command failed.",
            "cli_usage_error",
            message,
            vec![NextStep {
                action: "inspect_help".into(),
                command: help_command_for_path(help_path),
            }],
        );
        return format
            .render(&envelope)
            .unwrap_or_else(|_| message.to_string());
    }

    match render_help_for_path_with_format(help_path, Format::Table) {
        Ok(help_text) => help_text,
        Err(_) if !message.is_empty() => message.to_string(),
        Err(_) => error.to_string(),
    }
}

fn help_command_entry(command: &clap::Command) -> HelpCommandEntry {
    HelpCommandEntry {
        name: command.get_name().to_string(),
        summary: command
            .get_long_about()
            .or_else(|| command.get_about())
            .map(|value| value.to_string())
            .unwrap_or_default(),
    }
}

fn help_argument_entry(arg: &clap::Arg) -> HelpArgumentEntry {
    HelpArgumentEntry {
        name: positional_display(arg),
        help: arg_help(arg),
        required: arg.is_required_set(),
        defaults: arg_default_values(arg),
    }
}

fn help_option_entry(arg: &clap::Arg) -> HelpOptionEntry {
    HelpOptionEntry {
        flag: option_display(arg),
        help: arg_help(arg),
        required: arg.is_required_set(),
        defaults: arg_default_values(arg),
    }
}

fn positional_display(arg: &clap::Arg) -> String {
    if let Some(value_names) = arg.get_value_names() {
        value_names
            .iter()
            .map(|name| format!("<{}>", name))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        format!("<{}>", arg.get_id().as_str().to_ascii_uppercase())
    }
}

fn option_display(arg: &clap::Arg) -> String {
    let mut parts = Vec::new();
    if let Some(short) = arg.get_short() {
        parts.push(format!("-{short}"));
    }
    if let Some(long) = arg.get_long() {
        parts.push(format!("--{long}"));
    }

    let mut flag = if parts.is_empty() {
        arg.get_id().to_string()
    } else {
        parts.join(", ")
    };

    if arg
        .get_num_args()
        .map(|range| range.takes_values())
        .unwrap_or(false)
    {
        let values = if let Some(value_names) = arg.get_value_names() {
            value_names
                .iter()
                .map(|name| format!("<{}>", name))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            format!("<{}>", arg.get_id().as_str().to_ascii_uppercase())
        };
        flag = format!("{flag} {values}");
    }

    flag
}

fn arg_help(arg: &clap::Arg) -> String {
    arg.get_long_help()
        .or_else(|| arg.get_help())
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn arg_default_values(arg: &clap::Arg) -> Vec<String> {
    arg.get_default_values()
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

fn clap_error_message(error: &clap::Error) -> Option<String> {
    let rendered = error.to_string();
    let first_block = rendered.split("\n\n").next()?.trim();
    let message = first_block
        .strip_prefix("error: ")
        .unwrap_or(first_block)
        .trim();
    (!message.is_empty()).then(|| message.to_string())
}

fn append_help_section(rendered: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }

    rendered.push_str("\n\n");
    rendered.push_str(title);
    rendered.push('\n');
    for item in items {
        rendered.push_str("- ");
        rendered.push_str(item);
        rendered.push('\n');
    }
    while rendered.ends_with('\n') {
        rendered.pop();
        if !rendered.ends_with('\n') {
            break;
        }
    }
}

fn positional_args(args: &[String]) -> Vec<String> {
    let mut iter = args.iter().peekable();
    let mut positionals = Vec::new();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                let _ = iter.next();
            }
            "--json" => {}
            other if other.starts_with("--format=") => {}
            other if other.starts_with('-') => {}
            other => positionals.push(other.to_string()),
        }
    }

    positionals
}

pub fn help_surface_for_path(path: &[String]) -> Option<GuidanceSurface> {
    let key: Vec<&str> = path.iter().map(String::as_str).collect();
    if key.is_empty() {
        return Some(top_level_surface(crate::cli::top_level_help_metadata()));
    }

    let command_path = help_command_path_for_key(&key)?;
    if let Some(metadata) = crate::cli::command_group_help_metadata(command_path) {
        return Some(group_surface(metadata));
    }
    if let Some(metadata) = crate::cli::leaf_help_metadata(command_path) {
        return Some(leaf_surface(metadata));
    }

    None
}

pub fn runtime_success(
    command_path: &str,
    summary: impl Into<String>,
    next_steps: &[&str],
) -> String {
    let summary = summary.into();
    let envelope = crate::output::contract::ResultEnvelope::success(
        command_path,
        &summary,
        &serde_json::json!({}),
        runtime_steps_to_contract(next_steps),
    );
    match envelope.and_then(|envelope| current_output_format().render(&envelope)) {
        Ok(rendered) => rendered,
        Err(_) => summary,
    }
}

pub fn runtime_error(
    command_path: &str,
    summary: impl Into<String>,
    next_steps: &[&str],
) -> String {
    let summary = summary.into();
    let envelope = crate::output::contract::ResultEnvelope::error(
        command_path,
        "Command failed.",
        "runtime_error",
        &summary,
        runtime_steps_to_contract(next_steps),
    );
    current_output_format().render(&envelope).unwrap_or(summary)
}

fn current_output_format() -> Format {
    let args: Vec<String> = std::env::args().skip(1).collect();
    Format::detect_requested_format(&args)
}

fn runtime_steps_to_contract(next_steps: &[&str]) -> Vec<NextStep> {
    next_steps
        .iter()
        .map(|step| NextStep {
            action: "next_step".into(),
            command: (*step).to_string(),
        })
        .collect()
}

pub fn runtime_error_steps(command_path: &str, summary: &str) -> Vec<String> {
    let mut steps = Vec::new();

    if summary.contains("no default account set") {
        steps.push("Run telegram-agent-cli account list to inspect configured accounts.".into());
        steps.push(
            "Run telegram-agent-cli account use <name> to choose the default account.".into(),
        );
    }

    if command_path == "telegram-agent-cli account login" {
        steps.push(
            "Run telegram-agent-cli account login --help to inspect supported login flows.".into(),
        );
    }

    if steps.is_empty() {
        steps.push(
            "Use --help on the current command path to inspect prerequisites and examples.".into(),
        );
    }

    steps
}

pub fn runtime_error_next_steps(command_path: &str, summary: &str) -> Vec<NextStep> {
    runtime_error_steps(command_path, summary)
        .into_iter()
        .map(|step| {
            let command = if step.contains("account list") {
                "telegram-agent-cli account list"
            } else if step.contains("account use") {
                "telegram-agent-cli account use <name>"
            } else if step.contains("account login --help") {
                "telegram-agent-cli account login --help"
            } else {
                "telegram-agent-cli --help"
            };

            NextStep {
                action: "inspect_help".into(),
                command: command.into(),
            }
        })
        .collect()
}

pub fn repl_surface() -> GuidanceSurface {
    GuidanceSurface {
        guidance_kind: GuidanceKind::ReplHelp,
        command_path: Some("telegram-agent-cli repl".into()),
        summary: "Use the interactive chat REPL to send messages, inspect replies, and test bot behavior in one session.".into(),
        when_to_use: vec![
            "Use the REPL after you already know the account and chat you want to exercise.".into(),
        ],
        prerequisites: vec![
            "Ensure the selected account exists and the requested chat can be resolved before entering the REPL.".into(),
        ],
        actions: vec![
            "/send <text>: Send one text message in the active chat.".into(),
            "/recv [limit]: Inspect recent incoming messages (default 5).".into(),
            "/wait <text>: Wait for one incoming message containing the text.".into(),
            "/actions [msg_id]: Discover inline buttons, keyboards, and bot commands.".into(),
            "/click <button>: Click an inline button by label or callback data.".into(),
            "/trigger <action>: Trigger a discovered action by ID or label.".into(),
            "/unread: Show unread messages in the active chat.".into(),
            "/help: Show this help.".into(),
            "/exit: Leave the REPL session.".into(),
        ],
        examples: vec![
            "/send /start".into(),
            "/recv 5".into(),
            "/wait Welcome".into(),
            "/actions".into(),
            "/click Start".into(),
        ],
        next_steps: vec![
            "Run /help whenever you need to rediscover the interactive commands.".into(),
            "Run /exit to close the REPL when the session is complete.".into(),
        ],
        related_terms: Vec::new(),
        related_commands: vec![
            "telegram-agent-cli send --help".into(),
            "telegram-agent-cli message recv --help".into(),
            "telegram-agent-cli message wait --help".into(),
        ],
        status: None,
    }
}

fn help_command_path_for_key(key: &[&str]) -> Option<&'static str> {
    match key {
        [] => Some("telegram-agent-cli"),
        ["paths"] => Some("telegram-agent-cli paths"),
        ["context"] => Some("telegram-agent-cli context"),
        ["context", "show"] => Some("telegram-agent-cli context show"),
        ["account"] => Some("telegram-agent-cli account"),
        ["alias"] => Some("telegram-agent-cli alias"),
        ["list"] => Some("telegram-agent-cli list"),
        ["message"] => Some("telegram-agent-cli message"),
        ["peer"] => Some("telegram-agent-cli peer"),
        ["doctor"] => Some("telegram-agent-cli doctor"),
        ["export"] => Some("telegram-agent-cli export"),
        ["send"] => Some("telegram-agent-cli send"),
        ["send-file"] => Some("telegram-agent-cli send-file"),
        ["wait"] => Some("telegram-agent-cli wait"),
        ["run"] => Some("telegram-agent-cli run"),
        ["repl"] => Some("telegram-agent-cli repl"),
        ["mcp"] => Some("telegram-agent-cli mcp"),
        ["account", "add-user"] => Some("telegram-agent-cli account add-user"),
        ["account", "add-bot"] => Some("telegram-agent-cli account add-bot"),
        ["account", "list"] => Some("telegram-agent-cli account list"),
        ["account", "use"] => Some("telegram-agent-cli account use"),
        ["account", "login"] => Some("telegram-agent-cli account login"),
        ["account", "logout"] => Some("telegram-agent-cli account logout"),
        ["alias", "set"] => Some("telegram-agent-cli alias set"),
        ["alias", "list"] => Some("telegram-agent-cli alias list"),
        ["peer", "resolve"] => Some("telegram-agent-cli peer resolve"),
        ["list", "contacts"] => Some("telegram-agent-cli list contacts"),
        ["list", "chats"] => Some("telegram-agent-cli list chats"),
        ["message", "click-button"] => Some("telegram-agent-cli message click-button"),
        ["message", "list-actions"] => Some("telegram-agent-cli message list-actions"),
        ["message", "trigger-action"] => Some("telegram-agent-cli message trigger-action"),
        ["message", "recv"] => Some("telegram-agent-cli message recv"),
        ["message", "follow"] => Some("telegram-agent-cli message follow"),
        ["message", "wait"] => Some("telegram-agent-cli message wait"),
        ["message", "unread"] => Some("telegram-agent-cli message unread"),
        ["message", "forward"] => Some("telegram-agent-cli message forward"),
        ["message", "edit"] => Some("telegram-agent-cli message edit"),
        ["message", "pin"] => Some("telegram-agent-cli message pin"),
        ["message", "unpin"] => Some("telegram-agent-cli message unpin"),
        ["message", "download"] => Some("telegram-agent-cli message download"),
        ["send-photo"] => Some("telegram-agent-cli send-photo"),
        ["bot"] => Some("telegram-agent-cli bot"),
        ["bot", "set-commands"] => Some("telegram-agent-cli bot set-commands"),
        ["bot", "set-info"] => Some("telegram-agent-cli bot set-info"),
        _ => None,
    }
}

fn supported_output_formats() -> Vec<String> {
    vec![
        "table".into(),
        "yaml".into(),
        "toml".into(),
        "json".into(),
        "ndjson".into(),
    ]
}

fn default_behavior_for_path(path: &[String]) -> Vec<String> {
    let key: Vec<&str> = path.iter().map(String::as_str).collect();
    match key.as_slice() {
        [] => vec![
            "Default `--help` renders human-readable guidance.".into(),
            "Use `telegram-agent-cli help <command>` for structured help documents.".into(),
            "YAML is the default structured result format.".into(),
        ],
        ["paths"] => vec![
            "Reports the resolved config, data, state, and cache directories.".into(),
            "Marks each runtime directory as `default` or `override`.".into(),
        ],
        ["context"] | ["context", "show"] => vec![
            "Shows the persisted default account and any one-shot `--as` override.".into(),
            "Does not mutate the persisted default account.".into(),
        ],
        _ => vec!["YAML is the default structured result format.".into()],
    }
}

fn runtime_state_for_path(path: &[String]) -> Option<HelpRuntimeState> {
    let key: Vec<&str> = path.iter().map(String::as_str).collect();
    match key.as_slice() {
        ["paths"] => Some(HelpRuntimeState {
            directories: vec![
                "config".into(),
                "data".into(),
                "state".into(),
                "cache".into(),
            ],
            context_behavior: None,
        }),
        ["context"] | ["context", "show"] => Some(HelpRuntimeState {
            directories: Vec::new(),
            context_behavior: Some(
                "Reports the persisted default account plus any one-shot `--as` override without mutating stored state."
                    .into(),
            ),
        }),
        _ => None,
    }
}

fn top_level_surface(metadata: &crate::cli::TopLevelHelpMetadata) -> GuidanceSurface {
    GuidanceSurface {
        guidance_kind: GuidanceKind::TopLevelHelp,
        command_path: Some("telegram-agent-cli".into()),
        summary: metadata.summary.into(),
        when_to_use: metadata
            .when_to_use
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        prerequisites: metadata
            .prerequisites
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        actions: metadata
            .actions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        examples: Vec::new(),
        next_steps: metadata
            .next_steps
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        related_terms: Vec::new(),
        related_commands: Vec::new(),
        status: None,
    }
}

fn group_surface(metadata: &crate::cli::CommandGroupHelpMetadata) -> GuidanceSurface {
    GuidanceSurface {
        guidance_kind: GuidanceKind::CommandGroupHelp,
        command_path: Some(metadata.command_path.into()),
        summary: metadata.summary.into(),
        when_to_use: vec!["Use this surface to choose the right child command before executing a leaf command.".into()],
        prerequisites: vec!["If a child command acts on Telegram state, ensure the selected account and target context already exist.".into()],
        actions: metadata
            .actions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        examples: Vec::new(),
        next_steps: metadata
            .next_steps
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        related_terms: Vec::new(),
        related_commands: metadata
            .related_commands
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        status: None,
    }
}

fn leaf_surface(metadata: &crate::cli::LeafHelpMetadata) -> GuidanceSurface {
    GuidanceSurface {
        guidance_kind: GuidanceKind::LeafHelp,
        command_path: Some(metadata.command_path.into()),
        summary: metadata.summary.into(),
        when_to_use: vec!["Use this leaf command when you already know the target action and need a runnable invocation.".into()],
        prerequisites: metadata
            .prerequisites
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        actions: vec!["Review flags, confirm prerequisites, then run one of the example invocations.".into()],
        examples: metadata
            .examples
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        next_steps: metadata
            .next_steps
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        related_terms: Vec::new(),
        related_commands: metadata
            .next_steps
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        status: None,
    }
}
