use crate::errors::Result;
use crate::output::contract::{NextStep, ResultEnvelope};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Yaml,
    Toml,
    Json,
    Ndjson,
}

impl OutputFormat {
    pub fn from_flags(format: Option<&str>, json: bool) -> Result<Self> {
        if json {
            return Ok(Self::Json);
        }

        match format {
            Some(value) => Self::from_str(value),
            None => Ok(Self::Yaml),
        }
    }

    pub fn detect_requested_format(args: &[String]) -> Self {
        Self::detect_requested_format_or(args, Self::Yaml)
    }

    pub fn detect_requested_format_or(args: &[String], default: Self) -> Self {
        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--json" => return Self::Json,
                "--format" => {
                    if let Some(value) = iter.next() {
                        if let Ok(format) = Self::from_str(value) {
                            return format;
                        }
                    }
                }
                other if other.starts_with("--format=") => {
                    let value = other.trim_start_matches("--format=");
                    if let Ok(format) = Self::from_str(value) {
                        return format;
                    }
                }
                _ => {}
            }
        }
        default
    }

    pub fn render<T>(&self, value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        crate::output::render::render_serializable(value, *self)
    }

    pub fn print<T>(&self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        println!("{}", self.render(value)?);
        Ok(())
    }

    pub fn print_result<T>(
        &self,
        command: &str,
        summary: &str,
        data: &T,
        next_steps: Vec<NextStep>,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let envelope = ResultEnvelope::success(command, summary, data, next_steps)?;
        self.print(&envelope)
    }
}

impl FromStr for OutputFormat {
    type Err = crate::errors::TelegramCliError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "yaml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            "json" => Ok(Self::Json),
            "ndjson" => Ok(Self::Ndjson),
            _ => Err(crate::errors::TelegramCliError::Message(format!(
                "unsupported output format: {s}. Supported: table, yaml, toml, json, ndjson"
            ))),
        }
    }
}
