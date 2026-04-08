use crate::app::AppContext;
use crate::cli::ReplArgs;
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::{guidance, Format};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Context, Editor};

const REPL_COMMANDS: &[(&str, &str)] = &[
    ("/send ", "Send a message: /send <text>"),
    ("/recv ", "Receive recent messages: /recv [limit]"),
    ("/wait ", "Wait for a matching message: /wait <text>"),
    (
        "/actions",
        "List interactive actions: /actions [message_id]",
    ),
    ("/click ", "Click a button: /click <button>"),
    ("/trigger ", "Trigger an action: /trigger <action>"),
    ("/unread", "Show unread messages"),
    ("/help", "Show REPL help"),
    ("/exit", "Exit the REPL"),
    ("/quit", "Exit the REPL"),
];

struct ReplHelper;

impl rustyline::Helper for ReplHelper {}
impl rustyline::validate::Validator for ReplHelper {}
impl rustyline::highlight::Highlighter for ReplHelper {}
impl rustyline::hint::Hinter for ReplHelper {
    type Hint = String;
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if !line.starts_with('/') {
            return Ok((0, vec![]));
        }

        let prefix = &line[..pos];
        let candidates: Vec<Pair> = REPL_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(prefix))
            .map(|(cmd, help)| Pair {
                display: format!("{cmd}  -- {help}"),
                replacement: cmd.to_string(),
            })
            .collect();

        Ok((0, candidates))
    }
}

pub async fn run(context: &AppContext, args: ReplArgs) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.chat).await?;

    println!("{}", render_repl_banner(&peer.display_name, peer.peer_id));

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut rl = Editor::<ReplHelper, rustyline::history::DefaultHistory>::with_config(config)
        .map_err(|e| {
            crate::errors::TelegramCliError::Message(format!("readline init failed: {e}"))
        })?;
    rl.set_helper(Some(ReplHelper));

    let history_path = context.paths.state_dir.join("repl_history.txt");
    let _ = rl.load_history(&history_path);

    loop {
        match rl.readline(">>> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(&line);

                if input == "/exit" || input == "/quit" {
                    println!(
                        "{}",
                        render_repl_success(
                            "Interactive REPL session closed.",
                            &["Run telegram-agent-cli repl --help to reopen the REPL with a different chat context."],
                        )
                    );
                    break;
                }

                if input == "/help" {
                    show_help();
                    continue;
                }

                if let Some(cmd) = input.strip_prefix('/') {
                    handle_command(context, &args.as_account, peer.peer_id, cmd).await?;
                } else {
                    match context
                        .telegram
                        .send_text(&args.as_account, peer.peer_id, input, None, None)
                        .await
                    {
                        Ok(msg) => println!("{}", render_send_success(msg.message_id)),
                        Err(e) => println!("{}", render_repl_error(&e.to_string())),
                    }
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!(
                    "{}",
                    render_repl_success(
                        "Interactive REPL session closed.",
                        &["Run telegram-agent-cli repl --help to reopen the REPL with a different chat context."],
                    )
                );
                break;
            }
            Err(e) => {
                println!("{}", render_repl_error(&e.to_string()));
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);

    Ok(())
}

fn show_help() {
    let path = vec!["repl".to_string()];
    let table_args = vec!["--format".to_string(), "table".to_string()];
    match guidance::render_help_for_path(&path, &table_args) {
        Ok(rendered) => println!("{rendered}"),
        Err(_) => println!("{}", guidance::repl_surface().render_text()),
    }
}

async fn handle_command(
    context: &AppContext,
    account_name: &str,
    peer_id: i64,
    cmd: &str,
) -> Result<()> {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0];

    match command {
        "send" => {
            if parts.len() < 2 {
                println!("{}", render_usage_error("/send <text>"));
                return Ok(());
            }
            let text = parts[1];
            match context
                .telegram
                .send_text(account_name, peer_id, text, None, None)
                .await
            {
                Ok(msg) => println!("{}", render_send_success(msg.message_id)),
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "recv" => {
            let limit = if parts.len() > 1 {
                parts[1].parse().unwrap_or(5)
            } else {
                5
            };
            match context
                .telegram
                .recent_messages(account_name, peer_id, limit, None, false)
                .await
            {
                Ok(messages) => {
                    println!("{}", render_recv_result(messages)?);
                }
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "wait" => {
            if parts.len() < 2 {
                println!("{}", render_usage_error("/wait <text>"));
                return Ok(());
            }
            let text = parts[1];
            let timeout = std::time::Duration::from_secs(30);
            let filter = crate::telegram::MessageFilter {
                text_equals: None,
                text_contains: Some(text.to_string()),
                ..Default::default()
            };
            match context
                .telegram
                .wait_for_message(account_name, peer_id, &filter, timeout)
                .await
            {
                Ok(msg) => println!(
                    "{}",
                    render_wait_success(msg.text.as_deref().unwrap_or("<file>"))
                ),
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "actions" => {
            let message_id = if parts.len() > 1 {
                parts[1].parse::<i64>().ok()
            } else {
                None
            };
            match context
                .telegram
                .list_actions(account_name, peer_id, message_id, 20)
                .await
            {
                Ok(actions) => println!("{}", render_actions_result(actions)?),
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "click" => {
            if parts.len() < 2 {
                println!("{}", render_usage_error("/click <button>"));
                return Ok(());
            }
            let button = parts[1];
            let timeout = std::time::Duration::from_secs(5);
            match context
                .telegram
                .click_button(account_name, peer_id, button, None, timeout)
                .await
            {
                Ok(Some(msg)) => println!("{}", render_click_success(button, msg.text.as_deref())),
                Ok(None) => println!(
                    "{}",
                    render_repl_success(
                        format!("Button \"{button}\" clicked, no follow-up received."),
                        &["/recv 5"],
                    )
                ),
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "trigger" => {
            if parts.len() < 2 {
                println!("{}", render_usage_error("/trigger <action>"));
                return Ok(());
            }
            let action = parts[1];
            let timeout = std::time::Duration::from_secs(5);
            match context
                .telegram
                .trigger_action(account_name, peer_id, action, None, timeout)
                .await
            {
                Ok(result) => {
                    let json = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| format!("{result:?}"));
                    println!("{json}");
                }
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        "unread" => {
            match context
                .telegram
                .recent_messages(account_name, peer_id, 20, None, true)
                .await
            {
                Ok(messages) => {
                    if messages.is_empty() {
                        println!(
                            "{}",
                            guidance::runtime_success(
                                "telegram-agent-cli repl",
                                "No unread messages.",
                                &["/send <text>", "/wait <text>"],
                            )
                        );
                    } else {
                        println!("{}", render_recv_result(messages)?);
                    }
                }
                Err(e) => println!("{}", render_repl_error(&e.to_string())),
            }
        }
        _ => {
            println!("{}", render_unknown_command(command));
        }
    }

    Ok(())
}

fn render_repl_banner(chat_name: &str, peer_id: i64) -> String {
    let mut surface = guidance::repl_surface();
    surface.summary = format!(
        "Interactive REPL session is ready for chat {} (peer {}).",
        chat_name, peer_id
    );
    surface.render_text()
}

fn render_usage_error(usage: &str) -> String {
    render_repl_error_status(
        format!("Interactive command usage is invalid. Expected: {usage}"),
        &["Run /help to inspect the interactive command set."],
    )
}

fn render_unknown_command(command: &str) -> String {
    render_repl_error_status(
        format!("Unknown interactive command /{command}."),
        &["Run /help to inspect the interactive command set."],
    )
}

fn render_repl_error(summary: &str) -> String {
    render_repl_error_status(
        summary.to_string(),
        &[
            "Run /help to inspect the interactive command set.",
            "Run /exit to leave the REPL if the current chat context is wrong.",
        ],
    )
}

fn render_send_success(message_id: i64) -> String {
    render_repl_success(
        format!("Interactive message was sent with message id {message_id}."),
        &[
            "Run /recv 5 to inspect recent replies.",
            "Run /wait <text> to block on one matching reply.",
        ],
    )
}

fn render_wait_success(message_text: &str) -> String {
    render_repl_success(
        format!("Interactive wait matched the message: {message_text}"),
        &[
            "Run /recv 5 to inspect additional recent replies.",
            "Run /send <text> to continue the session.",
        ],
    )
}

fn render_click_success(button: &str, response_text: Option<&str>) -> String {
    let summary = match response_text {
        Some(text) => format!("Button \"{button}\" clicked. Response: {text}"),
        None => format!("Button \"{button}\" clicked."),
    };
    render_repl_success(summary, &["/recv 5", "/actions"])
}

fn render_actions_result(actions: Vec<crate::telegram::InteractiveAction>) -> Result<String> {
    let total = actions.len();
    let envelope = crate::output::contract::ResultEnvelope::success(
        "telegram-agent-cli repl actions",
        format!("Found {total} interactive action(s)."),
        &serde_json::json!({ "actions": actions, "total": total }),
        vec![
            NextStep {
                action: "click_button".into(),
                command: "/click <button>".into(),
            },
            NextStep {
                action: "trigger_action".into(),
                command: "/trigger <action>".into(),
            },
        ],
    )?;
    Format::Table.render(&envelope)
}

fn render_recv_result(messages: Vec<crate::telegram::IncomingMessage>) -> Result<String> {
    let total = messages.len();
    let preview: Vec<_> = messages.into_iter().take(3).collect();
    let envelope = crate::output::contract::ResultEnvelope::success(
        "telegram-agent-cli repl recv",
        "Interactive receive collected recent messages.",
        &serde_json::json!({
            "messages": preview,
            "total": total,
        }),
        vec![
            NextStep {
                action: "send_message".into(),
                command: "/send <text>".into(),
            },
            NextStep {
                action: "wait_for_reply".into(),
                command: "/wait <text>".into(),
            },
        ],
    )?;
    Format::Table.render(&envelope)
}

fn render_repl_success(summary: impl Into<String>, next_steps: &[&str]) -> String {
    let summary = summary.into();
    let envelope = crate::output::contract::ResultEnvelope::success(
        "telegram-agent-cli repl",
        &summary,
        &serde_json::json!({}),
        next_steps
            .iter()
            .map(|step| NextStep {
                action: "next_step".into(),
                command: (*step).to_string(),
            })
            .collect(),
    );
    match envelope.and_then(|envelope| Format::Table.render(&envelope)) {
        Ok(rendered) => rendered,
        Err(_) => summary,
    }
}

fn render_repl_error_status(summary: impl Into<String>, next_steps: &[&str]) -> String {
    let summary = summary.into();
    let envelope = crate::output::contract::ResultEnvelope::error(
        "telegram-agent-cli repl",
        "Command failed.",
        "runtime_error",
        &summary,
        next_steps
            .iter()
            .map(|step| NextStep {
                action: "next_step".into(),
                command: (*step).to_string(),
            })
            .collect(),
    );
    Format::Table.render(&envelope).unwrap_or(summary)
}

#[cfg(test)]
mod tests {
    use super::{
        handle_command, render_repl_banner, render_send_success, render_unknown_command,
        render_wait_success, ReplHelper,
    };
    use crate::app::AppContext;
    use crate::config::paths::{AppPaths, RuntimePathSource};
    use crate::storage::{AccountRepository, NewAccount, SecretStore};
    use crate::telegram::{MockTelegramAdapter, PeerKind, ResolvedPeer, TelegramAdapter};
    use rustyline::completion::Completer;
    use std::sync::Arc;

    fn test_context() -> (tempfile::TempDir, AppContext, Arc<MockTelegramAdapter>) {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config");
        let data_dir = temp.path().join("data");
        let state_dir = temp.path().join("state");
        let cache_dir = temp.path().join("cache");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let paths = AppPaths {
            config_dir: config_dir.clone(),
            data_dir: data_dir.clone(),
            state_dir,
            cache_dir,
            db_path: data_dir.join("state.sqlite"),
            master_key_path: config_dir.join("master.key"),
            config_source: RuntimePathSource::Override,
            data_source: RuntimePathSource::Override,
            state_source: RuntimePathSource::Override,
            cache_source: RuntimePathSource::Override,
        };
        let repo =
            AccountRepository::open(&paths.db_path, SecretStore::from_key_material("test-key"))
                .unwrap();
        repo.insert_account(NewAccount::user("alice", 1, "hash", "+10000000000"))
            .unwrap();
        repo.set_default_by_name("alice").unwrap();

        let telegram = Arc::new(MockTelegramAdapter::default());
        telegram.register_peer(
            "qa-bot",
            ResolvedPeer {
                peer_id: 42,
                peer_kind: PeerKind::Bot,
                display_name: "QA Bot".into(),
                username: Some("qa_bot".into()),
                packed_hex: Some("packed-qa-bot".into()),
            },
        );

        let adapter: Arc<dyn TelegramAdapter> = telegram.clone();
        let context = AppContext::new(paths, repo, adapter);
        (temp, context, telegram)
    }

    #[tokio::test]
    async fn handle_command_dispatches_send_without_leading_slash() {
        let (_temp, context, telegram) = test_context();

        handle_command(&context, "alice", 42, "send /start")
            .await
            .unwrap();

        let sent = telegram.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].text.as_deref(), Some("/start"));
    }

    #[test]
    fn repl_banner_uses_repl_guidance_contract() {
        let banner = render_repl_banner("QA Bot", 42);
        assert!(banner.contains("GUIDANCE_KIND: repl_help"));
        assert!(banner.contains("COMMAND_PATH: telegram-agent-cli repl"));
        assert!(banner.contains("ACTIONS:"));
        assert!(banner.contains("EXAMPLES:"));
    }

    #[test]
    fn repl_unknown_command_guidance_is_structured() {
        let guidance = render_unknown_command("nope");
        assert!(guidance.contains("COMMAND_PATH: telegram-agent-cli repl"));
        assert!(guidance.contains("STATUS: error"));
        assert!(guidance.contains("Unknown interactive command /nope."));
        assert!(!guidance.contains("\"status\""));
    }

    #[test]
    fn repl_history_path_lives_under_state_directory() {
        let (_temp, context, _telegram) = test_context();
        let history_path = context.paths.state_dir.join("repl_history.txt");
        assert!(history_path.starts_with(&context.paths.state_dir));
        assert_eq!(
            history_path.file_name().and_then(|value| value.to_str()),
            Some("repl_history.txt")
        );
    }

    #[test]
    fn repl_helper_completes_known_slash_commands() {
        let helper = ReplHelper;
        let history = rustyline::history::DefaultHistory::default();
        let ctx = rustyline::Context::new(&history);
        let (_start, candidates) = helper.complete("/he", 3, &ctx).unwrap();

        assert!(candidates
            .iter()
            .any(|candidate| candidate.replacement == "/help"));
    }

    #[test]
    fn repl_default_renderers_stay_human_readable() {
        let send = render_send_success(42);
        assert!(send.contains("COMMAND_PATH: telegram-agent-cli repl"));
        assert!(send.contains("SUMMARY: Interactive message was sent with message id 42."));
        assert!(!send.contains("\"status\""));

        let wait = render_wait_success("hello");
        assert!(wait.contains("Interactive wait matched the message: hello"));
        assert!(!wait.contains("\"status\""));
    }
}
