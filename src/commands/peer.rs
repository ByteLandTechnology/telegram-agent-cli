use crate::app::AppContext;
use crate::cli::{AliasCommand, AliasListArgs, AliasSetArgs, PeerCommand, PeerResolveArgs};
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::Format;

pub async fn run_peer(context: &AppContext, command: PeerCommand, format: Format) -> Result<()> {
    match command {
        PeerCommand::Resolve(args) => resolve_peer(context, args, format).await,
    }
}

pub async fn run_alias(context: &AppContext, command: AliasCommand, format: Format) -> Result<()> {
    match command {
        AliasCommand::Set(args) => set_alias(context, args, format).await,
        AliasCommand::List(args) => list_aliases(context, args, format),
    }
}

async fn resolve_peer(context: &AppContext, args: PeerResolveArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.query).await?;
    format.print_result(
        "telegram-agent-cli peer resolve",
        "Peer resolution completed.",
        &peer,
        vec![
            NextStep {
                action: "set_alias".into(),
                command: "telegram-agent-cli alias set <alias> <query>".into(),
            },
            NextStep {
                action: "inspect_help".into(),
                command: "telegram-agent-cli peer resolve --help".into(),
            },
        ],
    )
}

async fn set_alias(context: &AppContext, args: AliasSetArgs, format: Format) -> Result<()> {
    context.require_account(&args.as_account)?;
    let peer = context.resolve_peer(&args.as_account, &args.query).await?;
    context.repo.upsert_alias(
        &args.alias,
        peer.peer_id,
        peer.peer_kind,
        &peer.display_name,
        peer.username.as_deref(),
        peer.packed_hex.as_deref(),
    )?;
    format.print_result(
        "telegram-agent-cli alias set",
        &format!("Alias {} now points to peer {}.", args.alias, peer.peer_id),
        &serde_json::json!({
            "alias": args.alias,
            "peer_id": peer.peer_id,
        }),
        vec![
            NextStep {
                action: "inspect_aliases".into(),
                command: "telegram-agent-cli alias list".into(),
            },
            NextStep {
                action: "inspect_help".into(),
                command: "telegram-agent-cli peer resolve --help".into(),
            },
        ],
    )?;
    Ok(())
}

fn list_aliases(context: &AppContext, _args: AliasListArgs, format: Format) -> Result<()> {
    let aliases = context.repo.list_aliases()?;
    let total = aliases.len();
    format.print_result(
        "telegram-agent-cli alias list",
        "Alias list collected.",
        &AliasListOutput { aliases, total },
        vec![
            NextStep {
                action: "set_alias".into(),
                command: "telegram-agent-cli alias set <alias> <query>".into(),
            },
            NextStep {
                action: "resolve_peer".into(),
                command: "telegram-agent-cli peer resolve <query>".into(),
            },
        ],
    )
}

#[derive(serde::Serialize)]
struct AliasListOutput {
    aliases: Vec<crate::storage::AliasRecord>,
    total: usize,
}
