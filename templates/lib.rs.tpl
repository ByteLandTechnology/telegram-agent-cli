//! Baseline shared library for the generated package layout.
//! Optional feature overlays may add package-local support modules on top of
//! this baseline, but repository-owned CI and release automation remain
//! external to generated skill packages.

pub mod context;
pub mod help;

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;

/// Output format for serialization.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Yaml,
    Json,
    Toml,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Toml => "toml",
        }
    }
}

/// Minimal stable structured error contract.
#[derive(Debug, Clone, Serialize)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>,
    pub source: String,
    pub format: String,
}

impl StructuredError {
    pub fn new(code: &str, message: impl Into<String>, source: &str, format: Format) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
            source: source.to_string(),
            format: format.as_str().to_string(),
        }
    }

    pub fn with_detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details
            .get_or_insert_with(BTreeMap::new)
            .insert(key.to_string(), value.into());
        self
    }
}

/// Primary output structure for {{SKILL_NAME_SNAKE}}.
#[derive(Debug, Clone, Serialize)]
pub struct {{SKILL_NAME_PASCAL}}Output {
    pub status: String,
    pub message: String,
    pub input: String,
    pub effective_context: BTreeMap<String, String>,
}

/// Run the core logic and return the output.
pub fn run(input: &str, effective_context: BTreeMap<String, String>) -> {{SKILL_NAME_PASCAL}}Output {
    {{SKILL_NAME_PASCAL}}Output {
        status: "ok".to_string(),
        message: "Hello from {{SKILL_NAME}}".to_string(),
        input: input.to_string(),
        effective_context,
    }
}

/// Serialize a value to the given format and write to the writer.
pub fn serialize_value<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
    format: Format,
) -> Result<()> {
    match format {
        Format::Yaml => {
            let serialized = serde_yaml::to_string(value).context("failed to serialize as YAML")?;
            writer.write_all(serialized.as_bytes())?;
        }
        Format::Json => {
            serde_json::to_writer_pretty(&mut *writer, value)
                .context("failed to serialize as JSON")?;
            writeln!(writer)?;
        }
        Format::Toml => {
            let serialized =
                toml::to_string_pretty(value).context("failed to serialize as TOML")?;
            writer.write_all(serialized.as_bytes())?;
            writeln!(writer)?;
        }
    }

    Ok(())
}

/// Serialize a structured error in the selected output format.
pub fn write_structured_error<W: Write>(
    writer: &mut W,
    error: &StructuredError,
    format: Format,
) -> Result<()> {
    serialize_value(writer, error, format)
}
