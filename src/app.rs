use crate::cli::{Cli, Commands};
use crate::config::paths::AppPaths;
use crate::config::settings::ENV_MASTER_KEY;
use crate::errors::{Result, TelegramCliError};
use crate::output::contract::ActiveContextView;
use crate::storage::{AccountRepository, SecretStore};
use crate::telegram::{GrammersAdapter, PeerKind, ResolvedPeer, TelegramAdapter};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::Serialize;
use std::str::FromStr;
use std::sync::Arc;

pub struct AppContext {
    pub paths: AppPaths,
    pub repo: AccountRepository,
    pub telegram: Arc<dyn TelegramAdapter>,
}

pub async fn run(cli: Cli) -> Result<()> {
    let context = AppContext::bootstrap().await?;
    run_with_context(cli, &context).await
}

pub async fn run_with_context(cli: Cli, context: &AppContext) -> Result<()> {
    let format = crate::output::Format::from_str(&cli.format)?;
    match cli.command {
        Commands::Help(args) => {
            if args.command_path.len() == 1 && args.command_path[0] == "repl" {
                crate::output::guidance::print_human_help_for_path(&args.command_path)
            } else {
                let document =
                    crate::output::guidance::render_help_document_for_path(&args.command_path)?;
                format.print(&document)
            }
        }
        Commands::Paths(_) => crate::commands::paths::run(context, format),
        Commands::Context { command } => crate::commands::context::run(context, command, format),
        Commands::Account { command } => {
            crate::commands::account::run(context, command, format).await
        }
        Commands::Alias { command } => {
            crate::commands::peer::run_alias(context, command, format).await
        }
        Commands::Peer { command } => {
            crate::commands::peer::run_peer(context, command, format).await
        }
        Commands::List { command } => crate::commands::list::run(context, command, format).await,
        Commands::Doctor(mut args) => {
            if args.format.is_none() && !args.json {
                args.format = Some(cli.format.clone());
            }
            crate::commands::doctor::run_doctor(context, args)
        }
        Commands::Export(mut args) => {
            if args.format.is_none() {
                args.format = Some(cli.format.clone());
            }
            crate::commands::doctor::run_export(context, args)
        }
        Commands::Send(args) => crate::commands::message::send(context, args, format).await,
        Commands::SendFile(args) => {
            crate::commands::message::send_file(context, args, format).await
        }
        Commands::SendPhoto(args) => {
            crate::commands::message::send_photo(context, args, format).await
        }
        Commands::Wait(args) => crate::commands::message::wait(context, args, format).await,
        Commands::Message { command } => {
            crate::commands::message::run_message(context, command, format).await
        }
        Commands::Repl(args) => crate::commands::repl::run(context, args).await,
        Commands::Run(args) => crate::commands::scenario::run(context, args, format).await,
        Commands::Bot { command } => crate::commands::bot::run(context, command, format).await,
        Commands::Mcp(_) => crate::commands::mcp::run(context).await,
        Commands::Daemon { command } => {
            crate::commands::daemon::run(context, command, format).await
        }
    }
}

impl AppContext {
    pub fn new(
        paths: AppPaths,
        repo: AccountRepository,
        telegram: Arc<dyn TelegramAdapter>,
    ) -> Self {
        Self {
            paths,
            repo,
            telegram,
        }
    }

    async fn bootstrap() -> Result<Self> {
        let paths = AppPaths::detect()?;
        let key_material = load_master_key(&paths)?;

        let secrets = SecretStore::from_key_material(key_material.as_bytes());
        let repo = AccountRepository::open(&paths.db_path, secrets.clone())?;
        let telegram: Arc<dyn TelegramAdapter> =
            Arc::new(GrammersAdapter::new(paths.clone(), secrets));

        Ok(Self::new(paths, repo, telegram))
    }

    pub async fn resolve_peer(&self, account_name: &str, query: &str) -> Result<ResolvedPeer> {
        if let Some(alias) = self.repo.resolve_alias(query)? {
            return Ok(alias);
        }

        if let Ok(peer_id) = query.parse::<i64>() {
            return Ok(ResolvedPeer {
                peer_id,
                peer_kind: PeerKind::User,
                display_name: query.to_string(),
                username: None,
                packed_hex: None,
            });
        }

        self.telegram.resolve_peer(account_name, query).await
    }

    pub fn require_account(&self, account_name: &str) -> Result<()> {
        let actual_name = self.resolve_account_name(account_name)?;
        self.repo
            .find_account_by_name(&actual_name)?
            .ok_or_else(|| {
                crate::errors::TelegramCliError::Message(format!(
                    "account {account_name} was not found"
                ))
            })?;
        Ok(())
    }

    pub fn persisted_account_name(&self) -> Result<Option<String>> {
        self.repo.find_default_account_name()
    }

    pub fn preview_effective_account_name(
        &self,
        requested_account: Option<&str>,
    ) -> Result<Option<String>> {
        match requested_account {
            Some(name) if name != "default" => {
                self.repo.find_account_by_name(name)?.ok_or_else(|| {
                    TelegramCliError::Message(format!("account {name} was not found"))
                })?;
                Ok(Some(name.to_string()))
            }
            _ => self.persisted_account_name(),
        }
    }

    pub fn active_context_view(
        &self,
        requested_account: Option<&str>,
        requires_context: bool,
    ) -> Result<ActiveContextView> {
        let persisted_context = self.persisted_account_name()?;
        let effective_context = self.preview_effective_account_name(requested_account)?;
        let override_applied = matches!(
            requested_account,
            Some(requested) if requested != "default"
        ) && effective_context != persisted_context;

        Ok(ActiveContextView {
            persisted_context: persisted_context.clone(),
            effective_context,
            override_applied,
            mutation_path: "telegram-agent-cli account use <name>".into(),
            requires_context,
            set_default_hint: persisted_context
                .is_none()
                .then(|| "telegram-agent-cli account use <name>".into()),
        })
    }

    pub fn attach_active_context<T: Serialize>(
        &self,
        requested_account: Option<&str>,
        requires_context: bool,
        data: &T,
    ) -> Result<serde_json::Value> {
        let mut value = crate::output::contract::redact_serialized_value(data)?;
        let context =
            serde_json::to_value(self.active_context_view(requested_account, requires_context)?)
                .map_err(|error| {
                    TelegramCliError::Message(format!("failed to serialize context view: {error}"))
                })?;

        match &mut value {
            serde_json::Value::Object(map) => {
                map.insert("context".into(), context);
            }
            other => {
                let result = other.clone();
                let mut map = serde_json::Map::new();
                map.insert("result".into(), result);
                map.insert("context".into(), context);
                value = serde_json::Value::Object(map);
            }
        }

        Ok(value)
    }

    /// Resolve "default" to the actual default account name, or return the name as-is.
    pub fn resolve_account_name(&self, account_name: &str) -> Result<String> {
        self.preview_effective_account_name(Some(account_name))?
            .ok_or_else(|| {
                crate::errors::TelegramCliError::Message(
                    "no default account set; use `telegram-agent-cli account use <name>` to set one"
                        .into(),
                )
            })
    }
}

fn load_master_key(paths: &AppPaths) -> Result<String> {
    if let Ok(value) = std::env::var(ENV_MASTER_KEY) {
        return Ok(value);
    }

    if paths.master_key_path.exists() {
        return Ok(std::fs::read_to_string(&paths.master_key_path)?
            .trim()
            .to_string());
    }

    let mut raw_key = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw_key);
    let encoded = BASE64.encode(raw_key);
    std::fs::write(&paths.master_key_path, format!("{encoded}\n"))?;
    Ok(encoded)
}
