//! Optional REPL overlay for the generated package layout. This module is
//! package-local to generated skills when REPL support is enabled.

use crate::context::{
    build_context_state, inspect_context, load_active_context, parse_selector,
    persist_active_context, resolve_effective_context, ActiveContextState,
    InvocationContextOverrides, RuntimeLocations,
};
use crate::{
    run, serialize_value, write_structured_error, Format, {{SKILL_NAME_PASCAL}}Output, StructuredError,
};
use anyhow::Result;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context as RustylineContext, Editor, Helper};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct ReplHelper {
    candidates: Vec<String>,
}

impl ReplHelper {
    fn new(candidates: Vec<String>) -> Self {
        Self { candidates }
    }
}

impl Helper for ReplHelper {}
impl Hinter for ReplHelper {
    type Hint = String;
}
impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &RustylineContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos]
            .rfind(char::is_whitespace)
            .map(|index| index + 1)
            .unwrap_or(0);
        let needle = &line[start..pos];
        let matches = self
            .candidates
            .iter()
            .filter(|candidate| candidate.starts_with(needle))
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate.clone(),
            })
            .collect();
        Ok((start, matches))
    }
}

pub fn repl_help_text() -> String {
    [
        "REPL COMMANDS",
        "  help                         Show this plain-text REPL help",
        "  run <INPUT> [KEY=VALUE...]   Execute the run command with optional selectors",
        "  paths                        Show runtime directories",
        "  context show                 Show the persisted and effective Active Context",
        "  context use KEY=VALUE ...    Persist selectors as the Active Context",
        "  context use cwd=/path        Persist a current-directory cue",
        "  exit | quit                  End the session",
        "",
        "OUTPUT",
        "  Default REPL output is human-readable when the startup format is YAML.",
        "  Explicit JSON or TOML startup formats remain structured.",
    ]
    .join("\n")
}

pub fn completion_candidates(active_context: Option<&ActiveContextState>) -> Vec<String> {
    let mut candidates = vec![
        "help".to_string(),
        "run".to_string(),
        "paths".to_string(),
        "context".to_string(),
        "show".to_string(),
        "use".to_string(),
        "exit".to_string(),
        "quit".to_string(),
        "cwd=".to_string(),
    ];

    if let Some(active_context) = active_context {
        for (key, value) in &active_context.selectors {
            candidates.push(format!("{key}={value}"));
        }
        for (key, value) in &active_context.ambient_cues {
            candidates.push(format!("{key}={value}"));
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn render_run_output_for_repl(output: &{{SKILL_NAME_PASCAL}}Output) -> String {
    let mut text = format!(
        "status: {}\nmessage: {}\ninput: {}\n",
        output.status, output.message, output.input
    );
    if output.effective_context.is_empty() {
        text.push_str("effective_context: <none>\n");
    } else {
        text.push_str("effective_context:\n");
        for (key, value) in &output.effective_context {
            text.push_str(&format!("  {key}: {value}\n"));
        }
    }
    text
}

fn render_map_for_repl(title: &str, values: &BTreeMap<String, String>) -> String {
    let mut text = format!("{title}:\n");
    if values.is_empty() {
        text.push_str("  <none>\n");
        return text;
    }
    for (key, value) in values {
        text.push_str(&format!("  {key}: {value}\n"));
    }
    text
}

/// Start an interactive REPL session for {{SKILL_NAME}}.
pub fn start_repl(format: Format, runtime: RuntimeLocations) -> Result<()> {
    runtime.ensure_exists()?;

    let mut active_context = load_active_context(&runtime)?;
    let helper = ReplHelper::new(completion_candidates(active_context.as_ref()));
    let mut editor = Editor::<ReplHelper, DefaultHistory>::new()?;
    editor.set_helper(Some(helper));
    let history_path = runtime.history_file();
    let _ = editor.load_history(&history_path);

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    loop {
        write!(stderr, "{{SKILL_NAME}}> ")?;
        stderr.flush()?;

        let line = match editor.readline("") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(error) => return Err(error.into()),
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = editor.add_history_entry(trimmed);

        match trimmed {
            "exit" | "quit" => break,
            "help" => {
                writeln!(stdout, "{}", repl_help_text())?;
                stdout.flush()?;
            }
            "paths" => {
                if format == Format::Yaml {
                    let summary = runtime.summary();
                    writeln!(stdout, "config_dir: {}", summary.config_dir)?;
                    writeln!(stdout, "data_dir: {}", summary.data_dir)?;
                    writeln!(stdout, "state_dir: {}", summary.state_dir)?;
                    writeln!(stdout, "cache_dir: {}", summary.cache_dir)?;
                    if let Some(log_dir) = summary.log_dir {
                        writeln!(stdout, "log_dir: {log_dir}")?;
                    }
                } else {
                    serialize_value(&mut stdout, &runtime.summary(), format)?;
                }
                stdout.flush()?;
            }
            "context show" => {
                let inspection = inspect_context(&runtime, &InvocationContextOverrides::default())?;
                if format == Format::Yaml {
                    writeln!(stdout, "context_file: {}", inspection.context_file)?;
                    writeln!(
                        stdout,
                        "{}",
                        render_map_for_repl(
                            "effective_context",
                            &inspection.effective_context.effective_values,
                        )
                    )?;
                } else {
                    serialize_value(&mut stdout, &inspection, format)?;
                }
                stdout.flush()?;
            }
            command if command.starts_with("context use ") => {
                let mut selectors = BTreeMap::new();
                let mut cwd = None;
                for token in command.split_whitespace().skip(2) {
                    if let Some(path) = token.strip_prefix("cwd=") {
                        cwd = Some(PathBuf::from(path));
                    } else {
                        let (key, value) = parse_selector(token)?;
                        selectors.insert(key, value);
                    }
                }

                let state = build_context_state(None, selectors, cwd);
                let persisted = persist_active_context(&runtime, &state)?;
                active_context = Some(state);
                if let Some(helper) = editor.helper_mut() {
                    helper.candidates = completion_candidates(active_context.as_ref());
                }

                if format == Format::Yaml {
                    writeln!(stdout, "{}", persisted.message)?;
                } else {
                    serialize_value(&mut stdout, &persisted, format)?;
                }
                stdout.flush()?;
            }
            command if command.starts_with("run ") => {
                let mut parts = command.split_whitespace();
                let _ = parts.next();
                let input = parts.next().unwrap_or_default();
                let mut selectors = BTreeMap::new();
                for token in parts {
                    let (key, value) = parse_selector(token)?;
                    selectors.insert(key, value);
                }

                let effective_context = resolve_effective_context(
                    active_context.as_ref(),
                    &InvocationContextOverrides {
                        selectors,
                        current_directory: None,
                    },
                );
                let output = run(input, effective_context.effective_values);
                if format == Format::Yaml {
                    writeln!(stdout, "{}", render_run_output_for_repl(&output))?;
                } else {
                    serialize_value(&mut stdout, &output, format)?;
                }
                stdout.flush()?;
            }
            _ => {
                let error = StructuredError::new(
                    "repl.unknown_command",
                    format!("unknown REPL command: {trimmed}"),
                    "repl",
                    format,
                );
                write_structured_error(&mut stderr, &error, format)?;
                stderr.flush()?;
            }
        }
    }

    let _ = editor.save_history(&history_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{completion_candidates, render_run_output_for_repl, repl_help_text};
    use crate::context::build_context_state;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn repl_help_is_plain_text() {
        let help = repl_help_text();
        assert!(help.contains("REPL COMMANDS"));
        assert!(help.contains("context show"));
        assert!(!help.trim_start().starts_with('{'));
    }

    #[test]
    fn completion_candidates_include_visible_context_values() {
        let mut selectors = BTreeMap::new();
        selectors.insert("workspace".to_string(), "demo".to_string());
        let context = build_context_state(None, selectors, Some(PathBuf::from("/tmp/demo")));
        let candidates = completion_candidates(Some(&context));

        assert!(candidates.contains(&"run".to_string()));
        assert!(candidates.contains(&"workspace=demo".to_string()));
        assert!(candidates.contains(&"current_directory=/tmp/demo".to_string()));
    }

    #[test]
    fn yaml_repl_output_is_human_readable() {
        let mut effective_context = BTreeMap::new();
        effective_context.insert("workspace".to_string(), "demo".to_string());
        let output = crate::run("demo-input", effective_context);
        let rendered = render_run_output_for_repl(&output);

        assert!(rendered.contains("message: Hello from {{SKILL_NAME}}"));
        assert!(rendered.contains("workspace: demo"));
    }
}
