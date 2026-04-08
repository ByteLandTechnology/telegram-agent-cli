use crate::app::AppContext;
use crate::cli::{BotCommand, SetBotInfoArgs, SetCommandsArgs};
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::Format;

pub async fn run(context: &AppContext, command: BotCommand, format: Format) -> Result<()> {
    match command {
        BotCommand::SetCommands(args) => set_commands(context, args, format).await,
        BotCommand::SetInfo(args) => set_info(context, args, format).await,
    }
}

pub async fn set_commands(
    context: &AppContext,
    args: SetCommandsArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;

    let commands: Vec<(String, String)> = args
        .commands
        .split(',')
        .map(|pair| {
            let parts: Vec<&str> = pair.trim().splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(crate::errors::TelegramCliError::Message(format!(
                    "invalid command pair '{}', expected name|description",
                    pair.trim()
                )));
            }
            Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
        })
        .collect::<Result<Vec<_>>>()?;

    if commands.is_empty() {
        return Err(crate::errors::TelegramCliError::Message(
            "at least one command is required".into(),
        ));
    }

    context
        .telegram
        .set_bot_commands(&args.as_account, &commands)
        .await?;

    let output = serde_json::json!({
        "commands": commands.iter().map(|(name, desc)| serde_json::json!({
            "command": name,
            "description": desc,
        })).collect::<Vec<_>>(),
        "total": commands.len(),
    });
    format.print_result(
        "telegram-agent-cli bot set-commands",
        &format!("{} bot command(s) set.", commands.len()),
        &output,
        vec![NextStep {
            action: "set_bot_info".into(),
            command: "telegram-agent-cli bot set-info --as <account>".into(),
        }],
    )
}

pub async fn set_info(context: &AppContext, args: SetBotInfoArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;

    if args.description.is_none() && args.about.is_none() {
        return Err(crate::errors::TelegramCliError::Message(
            "at least one of --description or --about is required".into(),
        ));
    }

    context
        .telegram
        .set_bot_info(
            &args.as_account,
            args.description.as_deref(),
            args.about.as_deref(),
        )
        .await?;

    let output = serde_json::json!({
        "description": args.description,
        "about": args.about,
    });
    format.print_result(
        "telegram-agent-cli bot set-info",
        "Bot info updated.",
        &output,
        vec![NextStep {
            action: "set_bot_commands".into(),
            command: "telegram-agent-cli bot set-commands --as <account>".into(),
        }],
    )
}
