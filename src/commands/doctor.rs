use crate::app::AppContext;
use crate::cli::{DoctorArgs, ExportArgs};
use crate::errors::{Result, TelegramCliError};
use crate::output::contract::{NextStep, ResultEnvelope};
use crate::output::stream::render_ndjson_events;
use crate::output::Format;
use serde_json::json;

pub fn run_doctor(context: &AppContext, args: DoctorArgs) -> Result<()> {
    let format = Format::from_flags(args.format.as_deref(), args.json)?;
    let accounts = context.repo.list_accounts()?;
    let default_account = accounts
        .iter()
        .find(|account| account.is_default)
        .map(|account| account.name.clone());

    let payload = json!({
        "config_dir": context.paths.config_dir,
        "data_dir": context.paths.data_dir,
        "state_dir": context.paths.state_dir,
        "database": {
            "path": context.paths.db_path,
            "exists": context.paths.db_path.exists(),
        },
        "master_key_path": context.paths.master_key_path,
        "accounts": {
            "count": accounts.len(),
            "default": default_account,
        },
        "adapter": "grammers_stub",
    });

    let result = ResultEnvelope::success(
        "telegram-agent-cli doctor",
        "Environment diagnostics collected.",
        &payload,
        vec![NextStep {
            action: "inspect_accounts".into(),
            command: "telegram-agent-cli account list".into(),
        }],
    )?;

    format.print(&result)?;
    Ok(())
}

pub fn run_export(context: &AppContext, args: ExportArgs) -> Result<()> {
    let format = Format::from_flags(args.format.as_deref(), false)?;
    let run = if args.run_id == "latest" {
        context.repo.latest_run()?.ok_or_else(|| {
            TelegramCliError::Message("no scenario runs are available for export".into())
        })?
    } else {
        let run_id = args.run_id.parse::<i64>().map_err(|error| {
            TelegramCliError::Message(format!("invalid run id {}: {error}", args.run_id))
        })?;
        context
            .repo
            .find_run(run_id)?
            .ok_or_else(|| TelegramCliError::Message(format!("run {run_id} was not found")))?
    };

    let events = context.repo.list_run_events(run.id)?;
    let payload_items: Vec<serde_json::Value> = events
        .iter()
        .map(|event| {
            let payload = serde_json::from_str::<serde_json::Value>(&event.payload_json)
                .unwrap_or_else(|_| json!({ "raw": event.payload_json }));
            json!({
                "run_id": event.run_id,
                "step_name": event.step_name,
                "payload": payload,
                "created_at": event.created_at,
            })
        })
        .collect();

    if format == Format::Ndjson {
        let event_refs: Vec<(&str, &str, &serde_json::Value)> = payload_items
            .iter()
            .map(|item| ("run_event", "Scenario step exported.", item))
            .collect();
        println!(
            "{}",
            render_ndjson_events("telegram-agent-cli export", &event_refs, None)?
        );
        return Ok(());
    }

    let result = ResultEnvelope::success(
        "telegram-agent-cli export",
        "Scenario export complete.",
        &payload_items,
        vec![NextStep {
            action: "run_scenario".into(),
            command: "telegram-agent-cli run <path>".into(),
        }],
    )?;

    format.print(&result)?;
    Ok(())
}
