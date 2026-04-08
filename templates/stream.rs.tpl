//! Optional streaming overlay for the generated package layout. This module is
//! package-local to generated skills when streaming is enabled.

use crate::Format;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::io::{stdout, Write};

/// Streaming serializer for incremental output in {{SKILL_NAME}}.
pub struct StreamWriter<W: Write> {
    writer: W,
    format: Format,
}

impl<W: Write> StreamWriter<W> {
    /// Create a stream writer for the selected format.
    pub fn new(writer: W, format: Format) -> Result<Self> {
        if matches!(format, Format::Toml) {
            bail!("TOML does not support streaming output");
        }

        Ok(Self { writer, format })
    }

    /// Write one record using the configured framing protocol.
    pub fn write_record<T: Serialize>(&mut self, value: &T) -> Result<()> {
        match self.format {
            Format::Yaml => {
                let serialized =
                    serde_yaml::to_string(value).context("failed to serialize streamed YAML")?;
                let body = serialized.strip_prefix("---\n").unwrap_or(&serialized);

                self.writer.write_all(b"---\n")?;
                self.writer.write_all(body.as_bytes())?;
                if !body.ends_with('\n') {
                    self.writer.write_all(b"\n")?;
                }
            }
            Format::Json => {
                serde_json::to_writer(&mut self.writer, value)
                    .context("failed to serialize streamed JSON")?;
                self.writer.write_all(b"\n")?;
            }
            Format::Toml => bail!("TOML does not support streaming output"),
        }

        self.writer
            .flush()
            .context("failed to flush stream output")?;
        Ok(())
    }

    /// Finish the stream cleanly.
    pub fn finish(&mut self) -> Result<()> {
        if matches!(self.format, Format::Yaml) {
            self.writer.write_all(b"...\n")?;
        }

        self.writer
            .flush()
            .context("failed to flush stream output")?;
        Ok(())
    }
}

/// Stream every {{SKILL_NAME_PASCAL}} record to stdout using YAML multi-doc or NDJSON framing.
pub fn stream_values<I, T>(values: I, format: Format) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    let stdout = stdout();
    let mut handle = stdout.lock();
    let mut writer = StreamWriter::new(&mut handle, format)?;

    for value in values {
        if let Err(err) = writer.write_record(&value) {
            eprintln!("streaming error: {err}");
            return Err(err);
        }
    }

    if let Err(err) = writer.finish() {
        eprintln!("streaming error: {err}");
        return Err(err);
    }

    Ok(())
}

/// Stream a single {{SKILL_NAME_PASCAL}} value using the configured framing protocol.
pub fn stream_value<T: Serialize>(value: &T, format: Format) -> Result<()> {
    stream_values(std::iter::once(value), format)
}
