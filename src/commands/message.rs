use crate::app::AppContext;
use crate::cli::{
    ClickButtonArgs, DownloadArgs, EditMessageArgs, FollowArgs, ForwardArgs, ListActionsArgs,
    MessageCommand, PinArgs, RecvArgs, SendArgs, SendFileArgs, SendPhotoArgs, TriggerActionArgs,
    UnpinArgs, UnreadArgs, WaitArgs,
};
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::Format;
use crate::telegram::{InteractiveAction, MessageFilter, ReplyMarkupConfig};

fn contextual_output<T: serde::Serialize>(
    context: &AppContext,
    account_name: &str,
    data: &T,
) -> Result<serde_json::Value> {
    context.attach_active_context(Some(account_name), true, data)
}

pub async fn run_message(
    context: &AppContext,
    command: MessageCommand,
    format: Format,
) -> Result<()> {
    match command {
        MessageCommand::ClickButton(args) => click_button(context, args, format).await,
        MessageCommand::ListActions(args) => list_actions(context, args, format).await,
        MessageCommand::TriggerAction(args) => trigger_action(context, args, format).await,
        MessageCommand::Recv(args) => recv(context, args, format).await,
        MessageCommand::Follow(args) => follow(context, args, format).await,
        MessageCommand::Wait(args) => {
            wait_with_path(context, args, format, "telegram-agent-cli message wait").await
        }
        MessageCommand::Unread(args) => unread(context, args, format).await,
        MessageCommand::Forward(args) => forward(context, args, format).await,
        MessageCommand::Edit(args) => edit_message(context, args, format).await,
        MessageCommand::Pin(args) => pin_message(context, args, format).await,
        MessageCommand::Unpin(args) => unpin_message(context, args, format).await,
        MessageCommand::Download(args) => download_media(context, args, format).await,
    }
}

pub async fn send(context: &AppContext, args: SendArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.to).await?;
    let reply_markup = ReplyMarkupConfig::from_flags(
        args.reply_keyboard.as_deref(),
        args.inline_keyboard.as_deref(),
    )?;
    let message = context
        .telegram
        .send_text(
            &args.as_account,
            peer.peer_id,
            &args.text,
            args.reply_to,
            reply_markup.as_ref(),
        )
        .await?;
    let output = contextual_output(context, &args.as_account, &message)?;
    format.print_result(
        "telegram-agent-cli send",
        "Message sent.",
        &output,
        vec![NextStep {
            action: "read_replies".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn send_file(context: &AppContext, args: SendFileArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.to).await?;
    let reply_markup = ReplyMarkupConfig::from_flags(
        args.reply_keyboard.as_deref(),
        args.inline_keyboard.as_deref(),
    )?;
    let message = context
        .telegram
        .send_file(
            &args.as_account,
            peer.peer_id,
            args.path.as_path(),
            args.caption.as_deref(),
            args.reply_to,
            reply_markup.as_ref(),
        )
        .await?;
    let output = contextual_output(context, &args.as_account, &message)?;
    format.print_result(
        "telegram-agent-cli send-file",
        "File sent.",
        &output,
        vec![NextStep {
            action: "read_replies".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn send_photo(context: &AppContext, args: SendPhotoArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.to).await?;
    let reply_markup = ReplyMarkupConfig::from_flags(
        args.reply_keyboard.as_deref(),
        args.inline_keyboard.as_deref(),
    )?;
    let message = context
        .telegram
        .send_photo(
            &args.as_account,
            peer.peer_id,
            args.path.as_path(),
            args.caption.as_deref(),
            args.reply_to,
            reply_markup.as_ref(),
        )
        .await?;
    let output = contextual_output(context, &args.as_account, &message)?;
    format.print_result(
        "telegram-agent-cli send-photo",
        "Photo sent.",
        &output,
        vec![NextStep {
            action: "read_replies".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn forward(context: &AppContext, args: ForwardArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let from_peer = context.resolve_peer(&args.as_account, &args.from).await?;
    let to_peer = context.resolve_peer(&args.as_account, &args.to).await?;

    let message_ids: Vec<i32> = args
        .message_ids
        .split(',')
        .map(|s| {
            s.trim().parse::<i32>().map_err(|e| {
                crate::errors::TelegramCliError::Message(format!(
                    "invalid message ID '{}': {e}",
                    s.trim()
                ))
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if message_ids.is_empty() {
        return Err(crate::errors::TelegramCliError::Message(
            "at least one message ID is required".into(),
        ));
    }

    let forwarded_ids = context
        .telegram
        .forward_messages(
            &args.as_account,
            from_peer.peer_id,
            to_peer.peer_id,
            &message_ids,
        )
        .await?;

    let forwarded_count = forwarded_ids.len();
    let output = ForwardOutput {
        forwarded_count,
        forwarded_ids,
    };
    let output = contextual_output(context, &args.as_account, &output)?;
    format.print_result(
        "telegram-agent-cli message forward",
        &format!("{forwarded_count} message(s) forwarded."),
        &output,
        vec![NextStep {
            action: "inspect_forwarded".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn edit_message(
    context: &AppContext,
    args: EditMessageArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    context
        .telegram
        .edit_message(&args.as_account, peer.peer_id, args.message_id, &args.text)
        .await?;
    let output = contextual_output(
        context,
        &args.as_account,
        &serde_json::json!({
            "chat_id": peer.peer_id,
            "message_id": args.message_id,
            "text": args.text,
        }),
    )?;
    format.print_result(
        "telegram-agent-cli message edit",
        "Message edited.",
        &output,
        vec![NextStep {
            action: "verify_edit".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn pin_message(context: &AppContext, args: PinArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    context
        .telegram
        .pin_message(&args.as_account, peer.peer_id, args.message_id)
        .await?;
    let output = contextual_output(
        context,
        &args.as_account,
        &serde_json::json!({
            "chat_id": peer.peer_id,
            "message_id": args.message_id,
        }),
    )?;
    format.print_result(
        "telegram-agent-cli message pin",
        "Message pinned.",
        &output,
        vec![NextStep {
            action: "unpin_message".into(),
            command:
                "telegram-agent-cli message unpin --as <account> --chat <peer> --message-id <id>"
                    .into(),
        }],
    )
}

pub async fn unpin_message(context: &AppContext, args: UnpinArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    context
        .telegram
        .unpin_message(&args.as_account, peer.peer_id, args.message_id)
        .await?;
    let output = contextual_output(
        context,
        &args.as_account,
        &serde_json::json!({
            "chat_id": peer.peer_id,
            "message_id": args.message_id,
        }),
    )?;
    format.print_result(
        "telegram-agent-cli message unpin",
        "Message unpinned.",
        &output,
        vec![NextStep {
            action: "inspect_messages".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn download_media(
    context: &AppContext,
    args: DownloadArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;

    let output_path = match &args.output {
        Some(path) => path.clone(),
        None => std::env::current_dir().map_err(|e| {
            crate::errors::TelegramCliError::Message(format!(
                "failed to resolve current directory: {e}"
            ))
        })?,
    };

    let downloaded = context
        .telegram
        .download_media(
            &args.as_account,
            peer.peer_id,
            args.message_id,
            &output_path,
        )
        .await?;

    if downloaded {
        format.print_result(
            "telegram-agent-cli message download",
            "Media downloaded.",
            &serde_json::json!({
                "chat_id": peer.peer_id,
                "message_id": args.message_id,
                "output": output_path.display().to_string(),
            }),
            vec![],
        )
    } else {
        format.print_result(
            "telegram-agent-cli message download",
            "No downloadable media found in the message.",
            &serde_json::json!({
                "chat_id": peer.peer_id,
                "message_id": args.message_id,
                "downloaded": false,
            }),
            vec![],
        )
    }
}

pub async fn recv(context: &AppContext, args: RecvArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    let messages = context
        .telegram
        .recent_messages(
            &args.as_account,
            peer.peer_id,
            args.limit,
            args.offset_id,
            args.unread_only,
        )
        .await?;

    let output = MessagesOutput { messages };
    format.print_result(
        "telegram-agent-cli message recv",
        "Recent messages collected.",
        &output,
        vec![NextStep {
            action: "follow_replies".into(),
            command: "telegram-agent-cli message follow --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn follow(context: &AppContext, args: FollowArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    let filter = MessageFilter {
        text_equals: args.text.clone(),
        text_contains: args.text_contains.clone(),
        ..MessageFilter::default()
    };
    let timeout = parse_timeout(&args.timeout)?;

    let mut messages = Vec::new();
    for _ in 0..args.limit {
        let message = context
            .telegram
            .wait_for_message(&args.as_account, peer.peer_id, &filter, timeout)
            .await?;
        messages.push(message);
    }

    let output = MessagesOutput { messages };
    format.print_result(
        "telegram-agent-cli message follow",
        "Matching messages collected.",
        &output,
        vec![NextStep {
            action: "wait_for_one".into(),
            command: "telegram-agent-cli message wait --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn wait(context: &AppContext, args: WaitArgs, format: Format) -> Result<()> {
    wait_with_path(context, args, format, "telegram-agent-cli wait").await
}

async fn wait_with_path(
    context: &AppContext,
    args: WaitArgs,
    format: Format,
    command_path: &str,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    let filter = MessageFilter {
        text_equals: args.text.clone(),
        text_contains: args.text_contains.clone(),
        ..MessageFilter::default()
    };
    let timeout = parse_timeout(&args.timeout)?;

    let message = context
        .telegram
        .wait_for_message(&args.as_account, peer.peer_id, &filter, timeout)
        .await?;
    format.print_result(
        command_path,
        "Matching message received.",
        &message,
        vec![NextStep {
            action: "inspect_recent_messages".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub async fn click_button(
    context: &AppContext,
    args: ClickButtonArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;

    let timeout = parse_timeout(&args.wait_timeout)?;
    let response = context
        .telegram
        .click_button(
            &args.as_account,
            peer.peer_id,
            &args.button,
            args.message_id,
            timeout,
        )
        .await?;

    match response {
        Some(msg) => format.print_result(
            "telegram-agent-cli message click-button",
            "Inline button clicked and response received.",
            &msg,
            vec![NextStep {
                action: "inspect_recent_messages".into(),
                command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
            }],
        ),
        None => format.print_result(
            "telegram-agent-cli message click-button",
            "Inline button clicked, but no response arrived before timeout.",
            &serde_json::json!({
                "button": args.button,
                "message_id": args.message_id,
                "response_received": false
            }),
            vec![NextStep {
                action: "wait_for_followup".into(),
                command: "telegram-agent-cli message wait --as <account> --chat <peer>".into(),
            }],
        ),
    }
}

pub async fn list_actions(
    context: &AppContext,
    args: ListActionsArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    let actions = context
        .telegram
        .list_actions(&args.as_account, peer.peer_id, args.message_id, args.limit)
        .await?;
    let output = ActionListOutput { actions };
    format.print_result(
        "telegram-agent-cli message list-actions",
        "Interactive actions collected.",
        &output,
        vec![NextStep {
            action: "trigger_action".into(),
            command:
                "telegram-agent-cli message trigger-action --as <account> --chat <peer> <action>"
                    .into(),
        }],
    )
}

pub async fn trigger_action(
    context: &AppContext,
    args: TriggerActionArgs,
    format: Format,
) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;
    let timeout = parse_timeout(&args.wait_timeout)?;
    let result = context
        .telegram
        .trigger_action(
            &args.as_account,
            peer.peer_id,
            &args.action,
            args.message_id,
            timeout,
        )
        .await?;
    let summary = if result.response_received {
        "Interactive action triggered and response received."
    } else if result.url.is_some() {
        "Interactive action resolved to a URL target."
    } else {
        "Interactive action triggered, but no response arrived before timeout."
    };

    format.print_result(
        "telegram-agent-cli message trigger-action",
        summary,
        &result,
        vec![NextStep {
            action: "inspect_recent_messages".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer>".into(),
        }],
    )
}

pub fn parse_timeout(s: &str) -> Result<std::time::Duration> {
    humantime::parse_duration(s).map_err(|e| {
        crate::errors::TelegramCliError::Message(format!("invalid timeout duration: {e}"))
    })
}

pub async fn unread(context: &AppContext, args: UnreadArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;

    // Fetch recent messages and count unread (incoming only)
    let messages = context
        .telegram
        .recent_messages(&args.as_account, peer.peer_id, 1000, None, true)
        .await?;

    let unread_count = messages.len();
    let latest_message = messages.first().cloned();

    let output = UnreadOutput {
        chat_id: peer.peer_id,
        chat_name: peer.display_name,
        unread_count,
        latest_message,
    };
    format.print_result(
        "telegram-agent-cli message unread",
        "Unread message state collected.",
        &output,
        vec![NextStep {
            action: "read_unread_messages".into(),
            command: "telegram-agent-cli message recv --as <account> --chat <peer> --unread-only"
                .into(),
        }],
    )
}

#[derive(serde::Serialize)]
struct UnreadOutput {
    chat_id: i64,
    chat_name: String,
    unread_count: usize,
    latest_message: Option<crate::telegram::IncomingMessage>,
}

#[derive(serde::Serialize)]
struct MessagesOutput {
    messages: Vec<crate::telegram::IncomingMessage>,
}

#[derive(serde::Serialize)]
struct ActionListOutput {
    actions: Vec<InteractiveAction>,
}

#[derive(serde::Serialize)]
struct ForwardOutput {
    forwarded_count: usize,
    forwarded_ids: Vec<i64>,
}
