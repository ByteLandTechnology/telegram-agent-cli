use crate::errors::{Result, TelegramCliError};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceKind {
    TopLevelHelp,
    CommandGroupHelp,
    LeafHelp,
    ReplHelp,
    RuntimeSuccess,
    RuntimeError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct GuidanceSurface {
    pub guidance_kind: GuidanceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_path: Option<String>,
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub when_to_use: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub next_steps: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_terms: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GuidanceStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct HelpCommandEntry {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct HelpArgumentEntry {
    pub name: String,
    pub help: String,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub defaults: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct HelpOptionEntry {
    pub flag: String,
    pub help: String,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub defaults: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct HelpDocument {
    pub command: String,
    pub summary: String,
    pub usage: String,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_commands_as_map"
    )]
    pub commands: Vec<HelpCommandEntry>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_arguments_as_map"
    )]
    pub arguments: Vec<HelpArgumentEntry>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_options_as_map"
    )]
    pub options: Vec<HelpOptionEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before_you_run_it: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub try_next: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_behavior: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supported_output_formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_state: Option<HelpRuntimeState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct HelpRuntimeState {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_behavior: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct RuntimeDirectoryView {
    pub path: PathBuf,
    pub source: String,
    pub description: String,
    pub user_scoped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct RuntimePathsView {
    pub config: RuntimeDirectoryView,
    pub data: RuntimeDirectoryView,
    pub state: RuntimeDirectoryView,
    pub cache: RuntimeDirectoryView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct ActiveContextView {
    pub persisted_context: Option<String>,
    pub effective_context: Option<String>,
    pub override_applied: bool,
    pub mutation_path: String,
    pub requires_context: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_default_hint: Option<String>,
}

fn serialize_commands_as_map<S: Serializer>(
    entries: &[HelpCommandEntry],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(entries.len()))?;
    for entry in entries {
        let mut obj = Map::new();
        obj.insert("summary".into(), Value::String(entry.summary.clone()));
        map.serialize_entry(&entry.name, &obj)?;
    }
    map.end()
}

fn serialize_arguments_as_map<S: Serializer>(
    entries: &[HelpArgumentEntry],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(entries.len()))?;
    for entry in entries {
        map.serialize_entry(
            &entry.name,
            &help_value_object(&entry.help, entry.required, &entry.defaults),
        )?;
    }
    map.end()
}

fn serialize_options_as_map<S: Serializer>(
    entries: &[HelpOptionEntry],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(entries.len()))?;
    for entry in entries {
        map.serialize_entry(
            &entry.flag,
            &help_value_object(&entry.help, entry.required, &entry.defaults),
        )?;
    }
    map.end()
}

fn help_value_object(help: &str, required: bool, defaults: &[String]) -> Map<String, Value> {
    let mut obj = Map::new();
    obj.insert("help".into(), Value::String(help.to_string()));
    if required {
        obj.insert("required".into(), Value::Bool(true));
    }
    if !defaults.is_empty() {
        obj.insert("default".into(), Value::String(defaults.join(", ")));
    }
    obj
}

impl GuidanceSurface {
    pub fn render_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "GUIDANCE_KIND: {}",
            guidance_kind_label(&self.guidance_kind)
        ));
        if let Some(command_path) = &self.command_path {
            lines.push(format!("COMMAND_PATH: {command_path}"));
        }
        if let Some(status) = &self.status {
            lines.push(format!("STATUS: {}", guidance_status_label(status)));
        }
        lines.push(format!("SUMMARY: {}", self.summary));
        push_list_block(&mut lines, "WHEN_TO_USE", &self.when_to_use);
        push_list_block(&mut lines, "PREREQUISITES", &self.prerequisites);
        push_list_block(&mut lines, "ACTIONS", &self.actions);
        push_list_block(&mut lines, "EXAMPLES", &self.examples);
        push_list_block(&mut lines, "RELATED_COMMANDS", &self.related_commands);
        push_list_block(&mut lines, "NEXT_STEPS", &self.next_steps);
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct NextStep {
    pub action: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct ResultEnvelope {
    pub command: String,
    pub status: String,
    pub summary: String,
    pub data: Value,
    pub next_steps: Vec<NextStep>,
    pub errors: Vec<StructuredError>,
}

impl ResultEnvelope {
    pub fn success<T>(
        command: impl Into<String>,
        summary: impl Into<String>,
        data: &T,
        next_steps: Vec<NextStep>,
    ) -> Result<Self>
    where
        T: ?Sized + Serialize,
    {
        Ok(Self {
            command: command.into(),
            status: "ok".into(),
            summary: summary.into(),
            data: redact_serialized_value(data)?,
            next_steps,
            errors: Vec::new(),
        })
    }

    pub fn error(
        command: impl Into<String>,
        summary: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        next_steps: Vec<NextStep>,
    ) -> Self {
        Self {
            command: command.into(),
            status: "error".into(),
            summary: summary.into(),
            data: Value::Null,
            next_steps,
            errors: vec![StructuredError {
                code: code.into(),
                message: message.into(),
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct StreamEventEnvelope {
    pub command: String,
    pub event: String,
    pub sequence: u64,
    pub status: String,
    pub summary: String,
    pub data: Value,
    pub next_steps: Vec<NextStep>,
    #[serde(rename = "final")]
    pub final_: bool,
}

impl StreamEventEnvelope {
    pub fn ok<T>(
        command: impl Into<String>,
        event: impl Into<String>,
        sequence: u64,
        summary: impl Into<String>,
        data: &T,
        next_steps: Vec<NextStep>,
        final_event: bool,
    ) -> Result<Self>
    where
        T: ?Sized + Serialize,
    {
        Ok(Self {
            command: command.into(),
            event: event.into(),
            sequence,
            status: "ok".into(),
            summary: summary.into(),
            data: redact_serialized_value(data)?,
            next_steps,
            final_: final_event,
        })
    }
}

pub fn render_result_table(result: &ResultEnvelope) -> String {
    let mut lines = vec![
        format!("COMMAND_PATH: {}", result.command),
        format!("STATUS: {}", result.status),
        format!("SUMMARY: {}", result.summary),
        "DATA:".into(),
    ];
    lines.extend(render_value_block(&result.data, 0));
    push_next_steps(&mut lines, &result.next_steps);
    push_errors(&mut lines, &result.errors);
    lines.join("\n")
}

pub fn redact_serialized_value<T>(value: &T) -> Result<Value>
where
    T: ?Sized + Serialize,
{
    let value = serde_json::to_value(value)
        .map_err(|error| TelegramCliError::Message(format!("serialization failed: {error}")))?;
    Ok(redact_value(value))
}

pub fn redact_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[REDACTED]".into()))
                    } else {
                        (key, redact_value(value))
                    }
                })
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "phone", "token", "session", "password", "api_hash", "hash", "code", "qr",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn guidance_kind_label(kind: &GuidanceKind) -> &'static str {
    match kind {
        GuidanceKind::TopLevelHelp => "top_level_help",
        GuidanceKind::CommandGroupHelp => "command_group_help",
        GuidanceKind::LeafHelp => "leaf_help",
        GuidanceKind::ReplHelp => "repl_help",
        GuidanceKind::RuntimeSuccess => "runtime_success",
        GuidanceKind::RuntimeError => "runtime_error",
    }
}

fn guidance_status_label(status: &GuidanceStatus) -> &'static str {
    match status {
        GuidanceStatus::Ok => "ok",
        GuidanceStatus::Warning => "warning",
        GuidanceStatus::Error => "error",
    }
}

fn push_list_block(lines: &mut Vec<String>, label: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!("- {item}"));
    }
}

fn push_next_steps(lines: &mut Vec<String>, next_steps: &[NextStep]) {
    lines.push("NEXT_STEPS:".into());
    if next_steps.is_empty() {
        lines.push("- none".into());
        return;
    }
    for step in next_steps {
        lines.push(format!("- {} => {}", step.action, step.command));
    }
}

fn push_errors(lines: &mut Vec<String>, errors: &[StructuredError]) {
    lines.push("ERRORS:".into());
    if errors.is_empty() {
        lines.push("- none".into());
        return;
    }
    for error in errors {
        lines.push(format!("- {}: {}", error.code, error.message));
    }
}

fn render_value_block(value: &Value, indent: usize) -> Vec<String> {
    let prefix = "  ".repeat(indent);
    match value {
        Value::Null => vec![format!("{prefix}null")],
        Value::Bool(value) => vec![format!("{prefix}{value}")],
        Value::Number(value) => vec![format!("{prefix}{value}")],
        Value::String(value) => vec![format!("{prefix}{value}")],
        Value::Array(items) => {
            if items.is_empty() {
                return vec![format!("{prefix}[]")];
            }
            let mut lines = Vec::new();
            for item in items {
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        lines.push(format!("{prefix}-"));
                        lines.extend(render_value_block(item, indent + 1));
                    }
                    _ => {
                        let inline = render_value_block(item, 0).join(" ");
                        lines.push(format!("{prefix}- {inline}"));
                    }
                }
            }
            lines
        }
        Value::Object(map) => {
            if map.is_empty() {
                return vec![format!("{prefix}{{}}")];
            }
            let mut lines = Vec::new();
            for (key, value) in map {
                match value {
                    Value::Object(_) | Value::Array(_) => {
                        lines.push(format!("{prefix}{key}:"));
                        lines.extend(render_value_block(value, indent + 1));
                    }
                    _ => {
                        let inline = render_value_block(value, 0).join(" ");
                        lines.push(format!("{prefix}{key}: {inline}"));
                    }
                }
            }
            lines
        }
    }
}
