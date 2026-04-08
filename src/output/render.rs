use crate::errors::{Result, TelegramCliError};
use crate::output::contract::ResultEnvelope;
use crate::output::format::OutputFormat;
use serde::Serialize;
use serde_json::Value;

pub fn render_serializable<T>(value: &T, format: OutputFormat) -> Result<String>
where
    T: ?Sized + Serialize,
{
    match format {
        OutputFormat::Yaml => serde_yaml::to_string(value)
            .map(|rendered| insert_blank_lines_between_yaml_sections(&rendered))
            .map_err(|error| {
                TelegramCliError::Message(format!("failed to serialize yaml output: {error}"))
            }),
        OutputFormat::Toml => {
            let json = serde_json::to_value(value).map_err(|error| {
                TelegramCliError::Message(format!("output serialization failed: {error}"))
            })?;
            Ok(render_toml_ordered(&json))
        }
        OutputFormat::Json => serde_json::to_string_pretty(value).map_err(|error| {
            TelegramCliError::Message(format!("failed to serialize json output: {error}"))
        }),
        OutputFormat::Ndjson | OutputFormat::Table => {
            let value = serde_json::to_value(value).map_err(|error| {
                TelegramCliError::Message(format!("output serialization failed: {error}"))
            })?;
            render_value(&value, format)
        }
    }
}

pub fn render_value(value: &Value, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Yaml => serde_yaml::to_string(value)
            .map(|rendered| insert_blank_lines_between_yaml_sections(&rendered))
            .map_err(|error| {
                TelegramCliError::Message(format!("failed to serialize yaml output: {error}"))
            }),
        OutputFormat::Toml => Ok(render_toml_ordered(value)),
        OutputFormat::Json => serde_json::to_string_pretty(value).map_err(|error| {
            TelegramCliError::Message(format!("failed to serialize json output: {error}"))
        }),
        OutputFormat::Ndjson => render_ndjson(value),
        OutputFormat::Table => Ok(render_table_value(value)),
    }
}

fn render_ndjson(value: &Value) -> Result<String> {
    match value {
        Value::Array(items) => {
            let mut lines = Vec::with_capacity(items.len());
            for item in items {
                lines.push(serde_json::to_string(item).map_err(|error| {
                    TelegramCliError::Message(format!("failed to serialize ndjson row: {error}"))
                })?);
            }
            Ok(lines.join("\n"))
        }
        _ => serde_json::to_string(value).map_err(|error| {
            TelegramCliError::Message(format!("failed to serialize ndjson: {error}"))
        }),
    }
}

fn insert_blank_lines_between_yaml_sections(rendered: &str) -> String {
    let mut with_spacing = String::new();
    let mut seen_top_level_section = false;

    for line in rendered.lines() {
        let is_top_level_section = is_yaml_top_level_section(line);
        if is_top_level_section && seen_top_level_section && !with_spacing.ends_with("\n\n") {
            with_spacing.push('\n');
        }
        with_spacing.push_str(line);
        with_spacing.push('\n');
        if is_top_level_section {
            seen_top_level_section = true;
        }
    }

    with_spacing
}

fn is_yaml_top_level_section(line: &str) -> bool {
    !line.is_empty()
        && !line.starts_with([' ', '\t', '-'])
        && line.contains(':')
        && !matches!(line, "---" | "...")
}

pub fn render_table_value(value: &Value) -> String {
    if let Ok(envelope) = serde_json::from_value::<ResultEnvelope>(value.clone()) {
        return crate::output::contract::render_result_table(&envelope);
    }

    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| format!("{key}: {}", cell_value(value)))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Object(map) => map
                    .iter()
                    .map(|(key, value)| format!("{key}={}", cell_value(value)))
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => cell_value(item),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => cell_value(value),
    }
}

fn cell_value(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "<serialization-error>".into()),
    }
}

// ---------------------------------------------------------------------------
// Custom TOML renderer — uses dotted keys instead of [table] sections so that
// the output preserves the original field order from the struct definition.
// ---------------------------------------------------------------------------

fn render_toml_ordered(value: &Value) -> String {
    let mut lines = Vec::new();
    match value {
        Value::Object(map) => {
            let mut prev_was_compound = false;
            for (key, val) in map {
                let is_compound = matches!(val, Value::Object(_))
                    || matches!(val, Value::Array(arr) if arr.iter().any(Value::is_object));
                if (is_compound || prev_was_compound) && !lines.is_empty() {
                    lines.push(String::new());
                }
                toml_field(&mut lines, &toml_key(key), val);
                prev_was_compound = is_compound;
            }
        }
        _ => lines.push(toml_inline(value)),
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn toml_field(lines: &mut Vec<String>, prefix: &str, value: &Value) {
    match value {
        Value::Null => {}
        Value::String(s) => lines.push(format!("{prefix} = {}", toml_quote(s))),
        Value::Number(n) => lines.push(format!("{prefix} = {n}")),
        Value::Bool(b) => lines.push(format!("{prefix} = {b}")),
        Value::Array(arr) if arr.is_empty() => lines.push(format!("{prefix} = []")),
        Value::Array(arr) if arr.iter().all(|v| !v.is_object() && !v.is_array()) => {
            if arr.len() == 1 {
                lines.push(format!("{prefix} = [{}]", toml_inline(&arr[0])));
            } else {
                lines.push(format!("{prefix} = ["));
                for item in arr {
                    lines.push(format!("  {},", toml_inline(item)));
                }
                lines.push("]".into());
            }
        }
        Value::Array(arr) => {
            // Array of objects → inline tables
            lines.push(format!("{prefix} = ["));
            for item in arr {
                lines.push(format!("  {},", toml_inline_table(item)));
            }
            lines.push("]".into());
        }
        Value::Object(map) => {
            for (key, val) in map {
                toml_field(lines, &format!("{prefix}.{}", toml_key(key)), val);
            }
        }
    }
}

fn toml_inline(value: &Value) -> String {
    match value {
        Value::Null => "false".into(),
        Value::String(s) => toml_quote(s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(toml_inline).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(_) => toml_inline_table(value),
    }
}

fn toml_inline_table(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{} = {}", toml_key(k), toml_inline(v)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        _ => toml_inline(value),
    }
}

fn toml_key(key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        key.to_string()
    } else {
        toml_quote(key)
    }
}

fn toml_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}
