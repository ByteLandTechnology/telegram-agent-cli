use crate::app::AppContext;
use crate::automation::runner::ScenarioRunner;
use crate::cli::RunArgs;
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::Format;

pub async fn run(context: &AppContext, args: RunArgs, format: Format) -> Result<()> {
    let runner = ScenarioRunner::new(context);
    let run_id = runner.run_path(args.path.as_path()).await?;
    format.print_result(
        "telegram-agent-cli run",
        &format!("Scenario run {run_id} passed."),
        &serde_json::json!({
            "run_id": run_id,
            "status": "passed",
        }),
        vec![
            NextStep {
                action: "inspect_export".into(),
                command: format!("telegram-agent-cli export --run-id {run_id}"),
            },
            NextStep {
                action: "inspect_help".into(),
                command: "telegram-agent-cli run --help".into(),
            },
        ],
    )?;
    Ok(())
}
