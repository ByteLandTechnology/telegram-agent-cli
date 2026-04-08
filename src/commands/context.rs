use crate::app::AppContext;
use crate::cli::{ContextCommand, ContextShowArgs};
use crate::errors::Result;
use crate::output::contract::{ActiveContextView, NextStep};
use crate::output::Format;

pub fn run(context: &AppContext, command: ContextCommand, format: Format) -> Result<()> {
    match command {
        ContextCommand::Show(args) => show(context, args, format),
    }
}

fn show(context: &AppContext, args: ContextShowArgs, format: Format) -> Result<()> {
    let data: ActiveContextView = context.active_context_view(args.as_account.as_deref(), false)?;

    format.print_result(
        "telegram-agent-cli context show",
        "Active context collected.",
        &data,
        vec![
            NextStep {
                action: "set_default_context".into(),
                command: "telegram-agent-cli account use <name>".into(),
            },
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
        ],
    )
}
