use crate::errors::{Result, TelegramCliError};
use crate::output::contract::{NextStep, StreamEventEnvelope};
use serde::Serialize;

pub fn render_ndjson_events<T>(
    command: &str,
    events: &[(&str, &str, &T)],
    final_action: Option<NextStep>,
) -> Result<String>
where
    T: Serialize,
{
    let mut lines = Vec::with_capacity(events.len());
    let last_index = events.len().saturating_sub(1);
    for (index, (event_name, summary, data)) in events.iter().enumerate() {
        let payload = StreamEventEnvelope::ok(
            command,
            *event_name,
            index as u64 + 1,
            *summary,
            *data,
            if index == last_index {
                final_action.clone().into_iter().collect()
            } else {
                Vec::new()
            },
            index == last_index,
        )?;
        lines.push(serde_json::to_string(&payload).map_err(|error| {
            TelegramCliError::Message(format!("ndjson serialization failed: {error}"))
        })?);
    }
    Ok(lines.join("\n"))
}
