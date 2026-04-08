use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HelpOption {
    pub name: String,
    pub value_name: String,
    pub default_value: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelpSubcommand {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelpExample {
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExitCodeSpec {
    pub code: i32,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDirectoryHelp {
    pub config: String,
    pub data: String,
    pub state: String,
    pub cache: String,
    pub logs: String,
    pub scope: String,
    pub overrides: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveContextHelp {
    pub persisted_values: Vec<String>,
    pub ambient_cues: Vec<String>,
    pub inspection_command: String,
    pub switch_command: String,
    pub precedence_rule: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureAvailability {
    pub streaming: String,
    pub repl: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelpDocument {
    pub command_path: Vec<String>,
    pub purpose: String,
    pub usage: String,
    pub arguments: Vec<String>,
    pub options: Vec<HelpOption>,
    pub subcommands: Vec<HelpSubcommand>,
    pub output_formats: Vec<String>,
    pub exit_behavior: Vec<ExitCodeSpec>,
    pub runtime_directories: RuntimeDirectoryHelp,
    pub active_context: ActiveContextHelp,
    pub feature_availability: FeatureAvailability,
    #[serde(skip_serializing)]
    pub description: Vec<String>,
    #[serde(skip_serializing)]
    pub examples: Vec<HelpExample>,
}

fn runtime_directory_help() -> RuntimeDirectoryHelp {
    RuntimeDirectoryHelp {
        config: "User-authored configuration (user-scoped by default)".to_string(),
        data: "Durable CLI-managed business data".to_string(),
        state: "Recoverable runtime state, history, and persisted Active Context".to_string(),
        cache: "Disposable or reproducible artifacts".to_string(),
        logs: "Optional log path beneath state by default when logging is enabled".to_string(),
        scope: "user_scoped_default".to_string(),
        overrides: vec![
            "--config-dir".to_string(),
            "--data-dir".to_string(),
            "--state-dir".to_string(),
            "--cache-dir".to_string(),
            "--log-dir".to_string(),
        ],
    }
}

fn active_context_help() -> ActiveContextHelp {
    ActiveContextHelp {
        persisted_values: vec![
            "named profile label".to_string(),
            "selector key/value pairs".to_string(),
        ],
        ambient_cues: vec!["current_directory".to_string()],
        inspection_command: "{{SKILL_NAME}} context show".to_string(),
        switch_command: "{{SKILL_NAME}} context use --selector workspace=demo".to_string(),
        precedence_rule: "explicit invocation values override the persisted Active Context for one invocation only".to_string(),
    }
}

fn top_level_help() -> HelpDocument {
    HelpDocument {
        command_path: Vec::new(),
        purpose: "{{DESCRIPTION}}".to_string(),
        usage: "{{SKILL_NAME}} [OPTIONS] <COMMAND>".to_string(),
        arguments: Vec::new(),
        options: vec![
            HelpOption {
                name: "--format".to_string(),
                value_name: "yaml|json|toml".to_string(),
                default_value: "yaml".to_string(),
                description:
                    "Select the structured output format for one-shot commands and structured help"
                        .to_string(),
            },
            HelpOption {
                name: "--config-dir".to_string(),
                value_name: "PATH".to_string(),
                default_value: "platform default".to_string(),
                description: "Override the default configuration directory".to_string(),
            },
            HelpOption {
                name: "--data-dir".to_string(),
                value_name: "PATH".to_string(),
                default_value: "platform default".to_string(),
                description: "Override the default durable data directory".to_string(),
            },
            HelpOption {
                name: "--state-dir".to_string(),
                value_name: "PATH".to_string(),
                default_value: "derived from data".to_string(),
                description: "Override the runtime state directory".to_string(),
            },
            HelpOption {
                name: "--cache-dir".to_string(),
                value_name: "PATH".to_string(),
                default_value: "platform default".to_string(),
                description: "Override the cache directory".to_string(),
            },
            HelpOption {
                name: "--log-dir".to_string(),
                value_name: "PATH".to_string(),
                default_value: "state/logs when enabled".to_string(),
                description: "Override the optional log directory".to_string(),
            },
            HelpOption {
                name: "--help".to_string(),
                value_name: "-".to_string(),
                default_value: "false".to_string(),
                description: "Render plain-text help for the selected command path".to_string(),
            },
        ],
        subcommands: vec![
            HelpSubcommand {
                name: "run".to_string(),
                summary: "Execute the primary leaf command".to_string(),
            },
            HelpSubcommand {
                name: "paths".to_string(),
                summary: "Inspect runtime directory defaults and overrides".to_string(),
            },
            HelpSubcommand {
                name: "context".to_string(),
                summary: "Inspect or persist the Active Context".to_string(),
            },
            HelpSubcommand {
                name: "help".to_string(),
                summary: "Return machine-readable help for a command path".to_string(),
            },
        ],
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![
            ExitCodeSpec {
                code: 0,
                meaning: "Success or plain-text help".to_string(),
            },
            ExitCodeSpec {
                code: 2,
                meaning: "Structured usage or validation error".to_string(),
            },
        ],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "{{DESCRIPTION}}".to_string(),
            "This command surface reuses the approved description contract across Cargo metadata, SKILL.md, README, and help text.".to_string(),
            "Package-local packaging-ready support may appear only when enabled capabilities require it; repository-owned CI automation remains outside generated skill packages.".to_string(),
            "Available subcommands: run, paths, context, help.".to_string(),
        ],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} help run --format yaml".to_string(),
                description: "Inspect structured help for the run command".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} context use --selector workspace=demo".to_string(),
                description: "Persist an Active Context selector".to_string(),
            },
        ],
    }
}

fn run_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["run".to_string()],
        purpose: "Execute the generated leaf command".to_string(),
        usage: "{{SKILL_NAME}} run [OPTIONS] <INPUT>".to_string(),
        arguments: vec!["INPUT: required string payload for the run command".to_string()],
        options: vec![
            HelpOption {
                name: "--selector".to_string(),
                value_name: "KEY=VALUE".to_string(),
                default_value: "none".to_string(),
                description: "Apply an explicit per-invocation context override".to_string(),
            },
            HelpOption {
                name: "--cwd".to_string(),
                value_name: "PATH".to_string(),
                default_value: "none".to_string(),
                description: "Apply an explicit current-directory ambient cue".to_string(),
            },
            HelpOption {
                name: "--log-enabled".to_string(),
                value_name: "-".to_string(),
                default_value: "false".to_string(),
                description: "Expose the optional log directory for this invocation".to_string(),
            },
        ],
        subcommands: Vec::new(),
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![
            ExitCodeSpec {
                code: 0,
                meaning: "The command completed successfully".to_string(),
            },
            ExitCodeSpec {
                code: 2,
                meaning: "Missing input or other structured validation error".to_string(),
            },
        ],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "This is the primary leaf command used for validation.".to_string(),
            "Package-local packaging-ready support files may appear only when enabled capabilities require them; repository-owned CI automation stays outside generated skill packages.".to_string(),
            "If required input is missing, the command returns a structured error instead of raw help text.".to_string(),
        ],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} run demo-input".to_string(),
                description: "Execute the leaf command with the current Active Context".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} run demo-input --selector provider=staging".to_string(),
                description: "Override one context value for a single invocation".to_string(),
            },
        ],
    }
}

fn help_subcommand_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["help".to_string()],
        purpose: "Return machine-readable help for a command path".to_string(),
        usage: "{{SKILL_NAME}} help [COMMAND_PATH ...] [--format yaml|json|toml]".to_string(),
        arguments: vec![
            "COMMAND_PATH: optional command path such as run, paths, or context use".to_string(),
        ],
        options: Vec::new(),
        subcommands: Vec::new(),
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![
            ExitCodeSpec {
                code: 0,
                meaning: "The structured help document was returned".to_string(),
            },
            ExitCodeSpec {
                code: 2,
                meaning: "The requested help path was unknown".to_string(),
            },
        ],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "Use this subcommand when you need machine-readable command metadata.".to_string(),
        ],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} help run --format yaml".to_string(),
                description: "Inspect the run command as structured YAML".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} help context use --format json".to_string(),
                description: "Inspect a nested command path as structured JSON".to_string(),
            },
        ],
    }
}

fn paths_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["paths".to_string()],
        purpose: "Inspect runtime directory defaults and explicit overrides".to_string(),
        usage: "{{SKILL_NAME}} paths [OPTIONS]".to_string(),
        arguments: Vec::new(),
        options: vec![HelpOption {
            name: "--log-enabled".to_string(),
            value_name: "-".to_string(),
            default_value: "false".to_string(),
            description: "Include the optional log directory in the output".to_string(),
        }],
        subcommands: Vec::new(),
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![ExitCodeSpec {
            code: 0,
            meaning: "The runtime path summary was returned".to_string(),
        }],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "Use this command to inspect config, data, state, cache, and optional log locations."
                .to_string(),
        ],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} paths".to_string(),
                description: "Inspect the standard runtime directory family".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} paths --log-enabled".to_string(),
                description: "Inspect the optional log directory as well".to_string(),
            },
        ],
    }
}

fn context_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["context".to_string()],
        purpose: "Inspect or persist the Active Context".to_string(),
        usage: "{{SKILL_NAME}} context <COMMAND>".to_string(),
        arguments: Vec::new(),
        options: Vec::new(),
        subcommands: vec![
            HelpSubcommand {
                name: "show".to_string(),
                summary: "Display the current persisted and effective context".to_string(),
            },
            HelpSubcommand {
                name: "use".to_string(),
                summary: "Persist selectors and ambient cues as the Active Context".to_string(),
            },
        ],
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![
            ExitCodeSpec {
                code: 0,
                meaning: "Context command succeeded".to_string(),
            },
            ExitCodeSpec {
                code: 2,
                meaning: "Context input was incomplete".to_string(),
            },
        ],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "Available subcommands: show, use.".to_string(),
            "The Active Context is inspectable, persisted under state, and overridden explicitly per invocation."
                .to_string(),
        ],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} context show".to_string(),
                description: "Inspect the current effective context".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} context use --selector workspace=demo".to_string(),
                description: "Persist a selector for future invocations".to_string(),
            },
        ],
    }
}

fn context_show_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["context".to_string(), "show".to_string()],
        purpose: "Display the current Active Context".to_string(),
        usage: "{{SKILL_NAME}} context show".to_string(),
        arguments: Vec::new(),
        options: Vec::new(),
        subcommands: Vec::new(),
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![ExitCodeSpec {
            code: 0,
            meaning: "The current Active Context was returned".to_string(),
        }],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec![
            "Shows the persisted context file location and the effective values in use."
                .to_string(),
        ],
        examples: vec![HelpExample {
            command: "{{SKILL_NAME}} context show".to_string(),
            description: "Display the current Active Context".to_string(),
        }],
    }
}

fn context_use_help() -> HelpDocument {
    HelpDocument {
        command_path: vec!["context".to_string(), "use".to_string()],
        purpose: "Persist selectors and ambient cues as the Active Context".to_string(),
        usage: "{{SKILL_NAME}} context use [OPTIONS]".to_string(),
        arguments: Vec::new(),
        options: vec![
            HelpOption {
                name: "--name".to_string(),
                value_name: "NAME".to_string(),
                default_value: "none".to_string(),
                description: "Optional label for the persisted context".to_string(),
            },
            HelpOption {
                name: "--selector".to_string(),
                value_name: "KEY=VALUE".to_string(),
                default_value: "none".to_string(),
                description: "Persist one selector in the Active Context".to_string(),
            },
            HelpOption {
                name: "--cwd".to_string(),
                value_name: "PATH".to_string(),
                default_value: "none".to_string(),
                description: "Persist an ambient current-directory cue".to_string(),
            },
        ],
        subcommands: Vec::new(),
        output_formats: vec!["yaml".to_string(), "json".to_string(), "toml".to_string()],
        exit_behavior: vec![
            ExitCodeSpec {
                code: 0,
                meaning: "The context was persisted".to_string(),
            },
            ExitCodeSpec {
                code: 2,
                meaning: "No selectors or ambient cues were provided".to_string(),
            },
        ],
        runtime_directories: runtime_directory_help(),
        active_context: active_context_help(),
        feature_availability: FeatureAvailability {
            streaming: "optional add-on".to_string(),
            repl: "optional add-on".to_string(),
        },
        description: vec!["Persists one or more reusable selectors or ambient cues for future invocations."
            .to_string()],
        examples: vec![
            HelpExample {
                command: "{{SKILL_NAME}} context use --selector workspace=demo --selector provider=staging"
                    .to_string(),
                description: "Persist multiple selectors together".to_string(),
            },
            HelpExample {
                command: "{{SKILL_NAME}} context use --cwd /tmp/project".to_string(),
                description: "Persist a current-directory cue".to_string(),
            },
        ],
    }
}

pub fn structured_help(path: &[String]) -> Option<HelpDocument> {
    match path {
        [] => Some(top_level_help()),
        [one] if one == "help" => Some(help_subcommand_help()),
        [one] if one == "run" => Some(run_help()),
        [one] if one == "paths" => Some(paths_help()),
        [one] if one == "context" => Some(context_help()),
        [first, second] if first == "context" && second == "show" => Some(context_show_help()),
        [first, second] if first == "context" && second == "use" => Some(context_use_help()),
        _ => None,
    }
}

pub fn plain_text_help(path: &[String]) -> Option<String> {
    structured_help(path).map(|doc| render_plain_text_help(&doc))
}

pub fn render_plain_text_help(doc: &HelpDocument) -> String {
    let command_name = if doc.command_path.is_empty() {
        "{{SKILL_NAME}}".to_string()
    } else {
        format!("{{SKILL_NAME}} {}", doc.command_path.join(" "))
    };

    let mut out = String::new();
    out.push_str("NAME\n");
    out.push_str(&format!("  {} - {}\n\n", command_name, doc.purpose));

    out.push_str("SYNOPSIS\n");
    out.push_str(&format!("  {}\n\n", doc.usage));

    out.push_str("DESCRIPTION\n");
    for paragraph in &doc.description {
        out.push_str(&format!("  {}\n", paragraph));
    }
    if !doc.subcommands.is_empty() {
        out.push_str("  Available subcommands:\n");
        for subcommand in &doc.subcommands {
            out.push_str(&format!(
                "    {:<12} {}\n",
                subcommand.name, subcommand.summary
            ));
        }
    }
    out.push('\n');

    out.push_str("OPTIONS\n");
    if !doc.arguments.is_empty() {
        for argument in &doc.arguments {
            out.push_str(&format!("  {}\n", argument));
        }
    }
    if doc.options.is_empty() && doc.arguments.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for option in &doc.options {
            out.push_str(&format!(
                "  {:<18} {:<16} default: {:<18} {}\n",
                option.name, option.value_name, option.default_value, option.description
            ));
        }
    }
    out.push('\n');

    out.push_str("FORMATS\n");
    out.push_str(&format!(
        "  Structured formats: {}\n",
        doc.output_formats.join(", ")
    ));
    out.push_str(&format!(
        "  Streaming: {}\n",
        doc.feature_availability.streaming
    ));
    out.push_str(&format!("  REPL: {}\n\n", doc.feature_availability.repl));

    out.push_str("EXAMPLES\n");
    for example in &doc.examples {
        out.push_str(&format!("  {}\n", example.command));
        out.push_str(&format!("    {}\n", example.description));
    }
    out.push('\n');

    out.push_str("EXIT CODES\n");
    for exit_code in &doc.exit_behavior {
        out.push_str(&format!("  {}  {}\n", exit_code.code, exit_code.meaning));
    }

    out
}
