use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "telegram-agent-cli",
    disable_help_subcommand = true,
    version,
    about = "Telegram CLI for automation and bot testing",
    long_about = "Telegram CLI for automation and bot testing.\n\n\
        Manage accounts, send messages, interact with bots, run test scenarios,\n\
        and expose Telegram operations as MCP tools for AI agents.",
    after_long_help = "\x1b[1mGetting started:\x1b[0m\n  \
        $ telegram-agent-cli account add-bot --name mybot --token-env BOT_TOKEN\n  \
        $ telegram-agent-cli account login mybot\n  \
        $ telegram-agent-cli send --as mybot --to @user --text \"hello\"\n  \
        $ telegram-agent-cli repl --as mybot --chat @user\n  \
        $ telegram-agent-cli mcp                              # MCP server for AI agents"
)]
pub struct Cli {
    /// Output format: table, yaml, toml, json, or ndjson
    #[arg(long, global = true, default_value = "yaml")]
    pub format: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Show structured help for a command path.
    Help(HelpArgs),
    /// Show the resolved runtime directories used by telegram-agent-cli.
    Paths(PathsArgs),
    /// Inspect the persisted and effective account context.
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    /// Manage user and bot accounts.
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },
    /// Manage peer aliases.
    Alias {
        #[command(subcommand)]
        command: AliasCommand,
    },
    /// Resolve Telegram peers.
    Peer {
        #[command(subcommand)]
        command: PeerCommand,
    },
    /// List contacts, groups, and channels.
    List {
        #[command(subcommand)]
        command: ListCommand,
    },
    /// Show environment and storage diagnostics.
    Doctor(DoctorArgs),
    /// Export scenario run events.
    Export(ExportArgs),
    /// Send a message to a Telegram peer.
    Send(SendArgs),
    /// Send a file to a Telegram peer.
    SendFile(SendFileArgs),
    /// Send a photo to a Telegram peer.
    SendPhoto(SendPhotoArgs),
    /// Wait for a matching update from Telegram.
    Wait(WaitArgs),
    /// Manage messages.
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },
    /// Manage bot settings.
    Bot {
        #[command(subcommand)]
        command: BotCommand,
    },
    /// Run a scripted test scenario.
    Run(RunArgs),
    /// Interactive REPL mode for testing.
    Repl(ReplArgs),
    /// Start as an MCP tool server for AI agents.
    Mcp(McpArgs),
    /// Manage the background JSON-RPC daemon lifecycle.
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Debug, Args)]
pub struct HelpArgs {
    /// Command path to describe, such as `account add-bot`.
    #[arg(value_name = "COMMAND")]
    pub command_path: Vec<String>,
}

#[derive(Debug, Args, Default)]
pub struct PathsArgs {}

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Show the persisted and effective account context.
    Show(ContextShowArgs),
}

#[derive(Debug, Args)]
pub struct ContextShowArgs {
    /// Optional one-shot account override to preview as the effective context.
    #[arg(long = "as")]
    pub as_account: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReplArgs {
    /// Configured account name to act as during the REPL session.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Target chat, alias, username, or peer ID to open in the REPL.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
}

#[derive(Debug, Args)]
pub struct McpArgs {}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Start the background JSON-RPC daemon.
    Start(DaemonStartArgs),
    /// Stop the background JSON-RPC daemon.
    Stop(DaemonStopArgs),
    /// Restart the background JSON-RPC daemon.
    Restart(DaemonRestartArgs),
    /// Inspect the background JSON-RPC daemon state.
    Status(DaemonStatusArgs),
    #[command(name = "__serve", hide = true)]
    Serve(DaemonServeArgs),
}

#[derive(Debug, Args)]
pub struct DaemonStartArgs {
    /// How long to wait for the daemon to become ready.
    #[arg(long, default_value = "10s")]
    pub timeout: String,
}

#[derive(Debug, Args)]
pub struct DaemonStopArgs {
    /// How long to wait for the daemon to stop.
    #[arg(long, default_value = "10s")]
    pub timeout: String,
}

#[derive(Debug, Args)]
pub struct DaemonRestartArgs {
    /// How long to wait for stop/start transitions during restart.
    #[arg(long, default_value = "10s")]
    pub timeout: String,
}

#[derive(Debug, Args, Default)]
pub struct DaemonStatusArgs {}

#[derive(Debug, Args)]
pub struct DaemonServeArgs {
    #[arg(long, hide = true)]
    pub metadata_path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Click an inline button in a bot message.
    ClickButton(ClickButtonArgs),
    /// List interactive actions discovered from message markups and bot menus.
    ListActions(ListActionsArgs),
    /// Trigger an interactive action discovered from a message or bot menu.
    TriggerAction(TriggerActionArgs),
    /// Read recent messages from a Telegram peer.
    Recv(RecvArgs),
    /// Follow new messages from a Telegram peer.
    Follow(FollowArgs),
    /// Wait for a matching update from Telegram.
    Wait(WaitArgs),
    /// Show unread message statistics.
    Unread(UnreadArgs),
    /// Forward messages from one chat to another.
    Forward(ForwardArgs),
    /// Edit an existing message's text.
    Edit(EditMessageArgs),
    /// Pin a message in a chat.
    Pin(PinArgs),
    /// Unpin a message in a chat.
    Unpin(UnpinArgs),
    /// Download media from a message.
    Download(DownloadArgs),
}

#[derive(Debug, Args)]
pub struct ClickButtonArgs {
    /// Configured account name to use for the button click.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID that contains the target message.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Visible button label, callback data, or base64 callback data to click.
    pub button: String,
    /// Message ID containing the button when multiple messages have similar controls.
    #[arg(long)]
    pub message_id: Option<i64>,
    /// How long to wait for a follow-up bot response after clicking.
    #[arg(long, default_value = "5s")]
    pub wait_timeout: String,
}

#[derive(Debug, Args)]
pub struct ListActionsArgs {
    /// Configured account name to use when scanning for interactive actions.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID to inspect for interactive controls.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Limit the search to one specific message that contains interactive controls.
    #[arg(long)]
    pub message_id: Option<i64>,
    /// Number of recent messages to inspect for interactive markups.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct TriggerActionArgs {
    /// Configured account name to use when triggering the action.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID that contains the action.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Action ID or visible label returned by `telegram-agent-cli message list-actions`.
    pub action: String,
    /// Limit the action lookup to one specific message when needed.
    #[arg(long)]
    pub message_id: Option<i64>,
    /// How long to wait for a follow-up message after triggering the action.
    #[arg(long, default_value = "5s")]
    pub wait_timeout: String,
}

#[derive(Debug, Args)]
pub struct UnreadArgs {
    /// Configured account name to inspect unread state with.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID whose unread state you want to inspect.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Register a Telegram user account.
    AddUser(AddUserArgs),
    /// Register a Telegram bot account.
    AddBot(AddBotArgs),
    /// List configured accounts.
    List(AccountListArgs),
    /// Set the default account by name.
    Use(UseAccountArgs),
    /// Persist a session string for an account.
    Login(AccountLoginArgs),
    /// Clear a stored session for an account.
    Logout(AccountLogoutArgs),
}

#[derive(Debug, Args)]
pub struct AddUserArgs {
    /// Local account name to store in telegram-agent-cli for later `--as` selection.
    #[arg(long)]
    pub name: String,
    /// Telegram API ID from https://my.telegram.org.
    #[arg(long)]
    pub api_id: i32,
    /// Telegram API hash paired with the provided API ID.
    #[arg(long)]
    pub api_hash: String,
    /// Optional phone number to remember with the account profile.
    #[arg(long)]
    pub phone: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddBotArgs {
    /// Local account name to store in telegram-agent-cli for later `--as` selection.
    #[arg(long)]
    pub name: String,
    /// Bot token copied from BotFather.
    #[arg(long)]
    pub token: Option<String>,
    /// Environment variable name that contains the bot token.
    #[arg(long)]
    pub token_env: Option<String>,
    /// Optional Telegram API ID if the bot workflow also needs API credentials.
    #[arg(long)]
    pub api_id: Option<i32>,
    /// Optional Telegram API hash paired with `--api-id`.
    #[arg(long)]
    pub api_hash: Option<String>,
    /// Environment variable name that contains the API hash.
    #[arg(long)]
    pub api_hash_env: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountListArgs {}

#[derive(Debug, Args)]
pub struct UseAccountArgs {
    /// Name of the configured account to make the default for future commands.
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AccountLoginArgs {
    /// Name of the configured account whose session you want to store or refresh.
    pub name: String,
    /// Start QR-code login instead of session-string or code-based login.
    #[arg(long)]
    pub qr: bool,
    /// Session string to persist directly on the account.
    #[arg(long)]
    pub session: Option<String>,
    /// Environment variable name that contains the session string.
    #[arg(long)]
    pub session_env: Option<String>,
    /// One-time login code received from Telegram.
    #[arg(long)]
    pub code: Option<String>,
    /// Environment variable name that contains the login code.
    #[arg(long)]
    pub code_env: Option<String>,
    /// Two-factor password for accounts that require it.
    #[arg(long)]
    pub password: Option<String>,
    /// Environment variable name that contains the two-factor password.
    #[arg(long)]
    pub password_env: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountLogoutArgs {
    /// Name of the configured account whose stored session should be cleared.
    pub name: String,
}

#[derive(Debug, Subcommand)]
pub enum AliasCommand {
    /// Set or update an alias for a Telegram peer.
    Set(AliasSetArgs),
    /// List configured aliases.
    List(AliasListArgs),
}

#[derive(Debug, Subcommand)]
pub enum PeerCommand {
    /// Resolve a peer by alias, username, or numeric id.
    Resolve(PeerResolveArgs),
}

#[derive(Debug, Subcommand)]
pub enum ListCommand {
    /// List all contacts.
    Contacts(ContactListArgs),
    /// List all groups and channels.
    Chats(ChatListArgs),
}

#[derive(Debug, Args)]
pub struct AliasSetArgs {
    /// Configured account name to resolve the target peer with.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Local alias name to create or update.
    pub alias: String,
    /// Peer query such as a username, alias, phone number, or numeric ID.
    pub query: String,
}

#[derive(Debug, Args)]
pub struct AliasListArgs {}

#[derive(Debug, Args)]
pub struct PeerResolveArgs {
    /// Configured account name to resolve the peer with.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Peer query such as a username, alias, phone number, or numeric ID.
    pub query: String,
}

#[derive(Debug, Args)]
pub struct ContactListArgs {
    /// Configured account name whose contacts should be listed.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
}

#[derive(Debug, Args)]
pub struct ChatListArgs {
    /// Configured account name whose chats should be listed.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Shortcut for `--format json`.
    #[arg(long)]
    pub json: bool,
    /// Explicit output format for the diagnostics result.
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Run ID to export, or `latest` for the most recent run.
    #[arg(long = "run-id", default_value = "latest")]
    pub run_id: String,
    /// Explicit output format for exported run data.
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    /// Configured account name to send the message as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Target chat, alias, username, or peer ID to send to.
    #[arg(long, allow_hyphen_values = true)]
    pub to: String,
    /// Text message body to send.
    #[arg(long)]
    pub text: String,
    /// Reply to a specific message by its ID.
    #[arg(long)]
    pub reply_to: Option<i32>,
    /// Reply keyboard JSON: `[["Yes","No"],["Share:phone"]]`.
    /// Use `label:phone`, `label:geo`, `label:poll` for special buttons.
    #[arg(long = "reply-keyboard")]
    pub reply_keyboard: Option<String>,
    /// Inline keyboard JSON: `[["Click:callback:data"],["Open:url:https://example.com"]]`.
    #[arg(long = "inline-keyboard")]
    pub inline_keyboard: Option<String>,
}

#[derive(Debug, Args)]
pub struct SendFileArgs {
    /// Configured account name to send the file as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Target chat, alias, username, or peer ID to send to.
    #[arg(long, allow_hyphen_values = true)]
    pub to: String,
    /// Local filesystem path of the file to upload.
    pub path: PathBuf,
    /// Optional caption to attach to the uploaded file.
    #[arg(long)]
    pub caption: Option<String>,
    /// Reply to a specific message by its ID.
    #[arg(long)]
    pub reply_to: Option<i32>,
    /// Reply keyboard JSON: `[["Yes","No"],["Share:phone"]]`.
    #[arg(long = "reply-keyboard")]
    pub reply_keyboard: Option<String>,
    /// Inline keyboard JSON: `[["Click:callback:data"],["Open:url:https://example.com"]]`.
    #[arg(long = "inline-keyboard")]
    pub inline_keyboard: Option<String>,
}

#[derive(Debug, Args)]
pub struct SendPhotoArgs {
    /// Configured account name to send the photo as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Target chat, alias, username, or peer ID to send to.
    #[arg(long, allow_hyphen_values = true)]
    pub to: String,
    /// Local filesystem path of the photo to upload.
    pub path: PathBuf,
    /// Optional caption to attach to the uploaded photo.
    #[arg(long)]
    pub caption: Option<String>,
    /// Reply to a specific message by its ID.
    #[arg(long)]
    pub reply_to: Option<i32>,
    /// Reply keyboard JSON: `[["Yes","No"],["Share:phone"]]`.
    #[arg(long = "reply-keyboard")]
    pub reply_keyboard: Option<String>,
    /// Inline keyboard JSON: `[["Click:callback:data"],["Open:url:https://example.com"]]`.
    #[arg(long = "inline-keyboard")]
    pub inline_keyboard: Option<String>,
}

#[derive(Debug, Args)]
pub struct RecvArgs {
    /// Configured account name to read messages as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID to read from.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Maximum number of recent messages to return.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    /// Offset by message ID so pagination skips messages at or above this ID.
    #[arg(long)]
    pub offset_id: Option<i64>,
    /// Only show unread messages.
    #[arg(long)]
    pub unread_only: bool,
}

#[derive(Debug, Args)]
pub struct FollowArgs {
    /// Configured account name to stream messages as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID to follow.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Require an exact message text match.
    #[arg(long)]
    pub text: Option<String>,
    /// Require the message text to contain the provided substring.
    #[arg(long = "text-contains")]
    pub text_contains: Option<String>,
    /// Maximum time to keep waiting for matching messages.
    #[arg(long, default_value = "20s")]
    pub timeout: String,
    /// Stop after this many matching messages have been collected.
    #[arg(long, default_value_t = 1)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct WaitArgs {
    /// Configured account name to wait for messages as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID to wait on.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Require an exact message text match.
    #[arg(long)]
    pub text: Option<String>,
    /// Require the message text to contain the provided substring.
    #[arg(long = "text-contains")]
    pub text_contains: Option<String>,
    /// Maximum time to wait for one matching message.
    #[arg(long, default_value = "20s")]
    pub timeout: String,
}

#[derive(Debug, Args)]
pub struct ForwardArgs {
    /// Configured account name to forward messages as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Source chat, alias, username, or peer ID to forward from.
    #[arg(long, allow_hyphen_values = true)]
    pub from: String,
    /// Destination chat, alias, username, or peer ID to forward to.
    #[arg(long, allow_hyphen_values = true)]
    pub to: String,
    /// Comma-separated list of message IDs to forward.
    #[arg(long)]
    pub message_ids: String,
}

#[derive(Debug, Args)]
pub struct EditMessageArgs {
    /// Configured account name to edit the message as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID containing the message to edit.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Message ID to edit.
    #[arg(long)]
    pub message_id: i32,
    /// New text content for the message.
    #[arg(long)]
    pub text: String,
}

#[derive(Debug, Args)]
pub struct PinArgs {
    /// Configured account name to pin the message as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID containing the message to pin.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Message ID to pin.
    #[arg(long)]
    pub message_id: i32,
}

#[derive(Debug, Args)]
pub struct UnpinArgs {
    /// Configured account name to unpin the message as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID containing the message to unpin.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Message ID to unpin.
    #[arg(long)]
    pub message_id: i32,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Configured account name to download media as.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Chat, alias, username, or peer ID containing the message with media.
    #[arg(long, allow_hyphen_values = true)]
    pub chat: String,
    /// Message ID containing the media to download.
    #[arg(long)]
    pub message_id: i32,
    /// Local filesystem path to save the downloaded media to.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Local path to the scenario file that should be executed.
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum BotCommand {
    /// Set the bot command menu shown in the chat.
    SetCommands(SetCommandsArgs),
    /// Set the bot description and/or about text.
    SetInfo(SetBotInfoArgs),
}

#[derive(Debug, Args)]
pub struct SetCommandsArgs {
    /// Configured bot account name.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// Comma-separated list of bot commands in `name|description` format.
    /// Example: `/start|Start the bot,/help|Show help`
    #[arg(long)]
    pub commands: String,
}

#[derive(Debug, Args)]
pub struct SetBotInfoArgs {
    /// Configured bot account name.
    #[arg(long = "as", default_value = "default")]
    pub as_account: String,
    /// New bot description shown in the chat profile.
    #[arg(long)]
    pub description: Option<String>,
    /// New bot about text shown on the profile page.
    #[arg(long)]
    pub about: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct TopLevelHelpMetadata {
    pub summary: &'static str,
    pub when_to_use: &'static [&'static str],
    pub prerequisites: &'static [&'static str],
    pub actions: &'static [&'static str],
    pub next_steps: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct CommandGroupHelpMetadata {
    pub command_path: &'static str,
    pub summary: &'static str,
    pub actions: &'static [&'static str],
    pub next_steps: &'static [&'static str],
    pub related_commands: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct LeafHelpMetadata {
    pub command_path: &'static str,
    pub summary: &'static str,
    pub prerequisites: &'static [&'static str],
    pub examples: &'static [&'static str],
    pub next_steps: &'static [&'static str],
}

pub const TOP_LEVEL_HELP_METADATA: TopLevelHelpMetadata = TopLevelHelpMetadata {
    summary: "Explore Telegram accounts, peers, messages, diagnostics, scenarios, and interactive bot testing from one CLI.",
    when_to_use: &[
        "Start here when you need to discover the telegram-agent-cli capability map.",
        "Use the listed next steps to drill down into the correct command family.",
    ],
    prerequisites: &[
        "YAML is the default output. Use --format table for readable output or --format json for machine-readable envelopes.",
    ],
    actions: &[
        "help: Emit structured help documents for any command path. Use telegram-agent-cli help <command>.",
        "paths: Inspect config, data, state, and cache directories. Use telegram-agent-cli paths --help.",
        "context: Inspect the persisted default account and effective override behavior. Use telegram-agent-cli context --help.",
        "account: Manage users, bots, default account selection, and login state. Use telegram-agent-cli account --help.",
        "alias: Create and inspect saved peer aliases. Use telegram-agent-cli alias --help.",
        "peer: Resolve usernames, aliases, and IDs into peers. Use telegram-agent-cli peer --help.",
        "list: Inspect contacts, groups, and channels. Use telegram-agent-cli list --help.",
        "message: Read, follow, wait for, or interact with messages. Use telegram-agent-cli message --help.",
        "doctor: Inspect local configuration and storage state. Use telegram-agent-cli doctor --help.",
        "export: Export scenario run events. Use telegram-agent-cli export --help.",
        "send/send-file/send-photo/wait: Execute direct chat actions. Use telegram-agent-cli send --help, telegram-agent-cli send-file --help, telegram-agent-cli send-photo --help, or telegram-agent-cli wait --help.",
        "bot: Configure bot command menus and profile info. Use telegram-agent-cli bot --help.",
        "run: Execute a stored scenario file. Use telegram-agent-cli run --help.",
        "repl: Open an interactive testing session. Use telegram-agent-cli repl --help.",
        "mcp: Start as an MCP tool server for AI agents. Use telegram-agent-cli mcp --help.",
        "daemon: Manage the background JSON-RPC daemon. Use telegram-agent-cli daemon --help.",
    ],
    next_steps: &[
        "telegram-agent-cli help account add-bot",
        "telegram-agent-cli paths --help",
        "telegram-agent-cli context --help",
        "telegram-agent-cli account --help",
        "telegram-agent-cli message --help",
        "telegram-agent-cli doctor --help",
        "telegram-agent-cli repl --help",
        "telegram-agent-cli daemon --help",
    ],
};

pub const COMMAND_GROUP_HELP_METADATA: &[CommandGroupHelpMetadata] = &[
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli context",
        summary: "Inspect the persisted default account and the effective account selected for the current invocation.",
        actions: &[
            "Choose context show to inspect persisted and effective account selection without mutating the default account.",
        ],
        next_steps: &["telegram-agent-cli context show --help"],
        related_commands: &["telegram-agent-cli account use --help", "telegram-agent-cli account list --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli account",
        summary: "Manage Telegram user and bot accounts, default account selection, and login state.",
        actions: &[
            "Choose account add-user when registering a user identity with API credentials.",
            "Choose account add-bot when registering a bot token-backed identity.",
            "Choose account list to inspect configured accounts and login state.",
            "Choose account use to change the default account for later commands.",
            "Choose account login or logout to manage active sessions.",
        ],
        next_steps: &[
            "telegram-agent-cli account add-user --help",
            "telegram-agent-cli account add-bot --help",
            "telegram-agent-cli account list --help",
            "telegram-agent-cli account use --help",
            "telegram-agent-cli account login --help",
            "telegram-agent-cli account logout --help",
        ],
        related_commands: &["telegram-agent-cli --help", "telegram-agent-cli doctor --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli alias",
        summary: "Create and inspect saved peer aliases so later commands can target chats without raw IDs.",
        actions: &[
            "Choose alias set to bind a memorable name to a resolved peer.",
            "Choose alias list to inspect stored alias mappings.",
        ],
        next_steps: &["telegram-agent-cli alias set --help", "telegram-agent-cli alias list --help"],
        related_commands: &["telegram-agent-cli peer --help", "telegram-agent-cli list --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli list",
        summary: "Inspect available contacts, groups, and channels for the selected account.",
        actions: &[
            "Choose list contacts to inspect direct-contact targets.",
            "Choose list chats to inspect group and channel targets.",
        ],
        next_steps: &["telegram-agent-cli list contacts --help", "telegram-agent-cli list chats --help"],
        related_commands: &["telegram-agent-cli peer --help", "telegram-agent-cli alias --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli message",
        summary: "Read, follow, wait for, inspect, or interact with existing message traffic in a chat.",
        actions: &[
            "Choose message recv to read recent messages from a chat.",
            "Choose message follow to stream new matching messages for a bounded period.",
            "Choose message wait to block until one matching message arrives.",
            "Choose message unread to inspect unread counts and latest unread content.",
            "Choose message forward to copy messages from one chat to another.",
            "Choose message edit to update the text of an existing message.",
            "Choose message pin or unpin to manage pinned messages in a chat.",
            "Choose message download to save media from a message to a local file.",
            "Choose message click-button to interact with inline buttons in bot messages.",
            "Choose message list-actions to discover inline buttons, reply keyboard buttons, and bot menu commands.",
            "Choose message trigger-action to replay an action discovered from list-actions.",
        ],
        next_steps: &[
            "telegram-agent-cli message recv --help",
            "telegram-agent-cli message follow --help",
            "telegram-agent-cli message wait --help",
            "telegram-agent-cli message unread --help",
            "telegram-agent-cli message forward --help",
            "telegram-agent-cli message edit --help",
            "telegram-agent-cli message pin --help",
            "telegram-agent-cli message unpin --help",
            "telegram-agent-cli message download --help",
            "telegram-agent-cli message click-button --help",
            "telegram-agent-cli message list-actions --help",
            "telegram-agent-cli message trigger-action --help",
        ],
        related_commands: &["telegram-agent-cli send --help", "telegram-agent-cli wait --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli peer",
        summary: "Resolve usernames, aliases, and numeric IDs into concrete Telegram peers.",
        actions: &[
            "Choose peer resolve to confirm the peer kind, ID, and display target for later commands.",
        ],
        next_steps: &["telegram-agent-cli peer resolve --help"],
        related_commands: &["telegram-agent-cli alias --help", "telegram-agent-cli list --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli bot",
        summary: "Manage bot profile metadata and command menus.",
        actions: &[
            "Choose bot set-commands to publish the bot command menu shown in chat.",
            "Choose bot set-info to update the bot description and about text.",
        ],
        next_steps: &[
            "telegram-agent-cli bot set-commands --help",
            "telegram-agent-cli bot set-info --help",
        ],
        related_commands: &["telegram-agent-cli account --help", "telegram-agent-cli message --help"],
    },
    CommandGroupHelpMetadata {
        command_path: "telegram-agent-cli daemon",
        summary: "Manage the background JSON-RPC daemon that exposes the same MCP tool surface without keeping a foreground terminal attached.",
        actions: &[
            "Choose daemon start to launch the managed background service.",
            "Choose daemon status to inspect whether the daemon is running and where it is listening.",
            "Choose daemon stop or restart to recover the managed background service from the CLI.",
        ],
        next_steps: &[
            "telegram-agent-cli daemon start --help",
            "telegram-agent-cli daemon status --help",
            "telegram-agent-cli daemon stop --help",
            "telegram-agent-cli daemon restart --help",
        ],
        related_commands: &["telegram-agent-cli mcp --help", "telegram-agent-cli doctor --help"],
    },
];

pub const LEAF_HELP_METADATA: &[LeafHelpMetadata] = &[
    LeafHelpMetadata {
        command_path: "telegram-agent-cli paths",
        summary: "Inspect the resolved config, data, state, and cache directories used by telegram-agent-cli.",
        prerequisites: &["Use this command when you need to verify where telegram-agent-cli is storing configuration, runtime state, or caches."],
        examples: &["telegram-agent-cli paths", "telegram-agent-cli paths --format json"],
        next_steps: &["telegram-agent-cli context show", "telegram-agent-cli doctor"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli context show",
        summary: "Inspect the persisted default account, the effective account for this invocation, and one-shot override behavior.",
        prerequisites: &["Use this command after configuring one or more accounts if you need to confirm which account will be applied."],
        examples: &["telegram-agent-cli context show", "telegram-agent-cli context show --as bot --format json"],
        next_steps: &["telegram-agent-cli account list", "telegram-agent-cli account use --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli doctor",
        summary: "Inspect local telegram-agent-cli configuration, storage paths, adapter state, and account summary.",
        prerequisites: &["Ensure telegram-agent-cli can resolve its config, data, and state directories."],
        examples: &["telegram-agent-cli doctor", "telegram-agent-cli doctor --json", "telegram-agent-cli doctor --format table"],
        next_steps: &["telegram-agent-cli account list", "telegram-agent-cli export --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli export",
        summary: "Export scenario run events as readable results or NDJSON stream events.",
        prerequisites: &["Ensure at least one scenario run exists before exporting latest output."],
        examples: &[
            "telegram-agent-cli export --run-id latest",
            "telegram-agent-cli export --run-id latest --format json",
            "telegram-agent-cli export --run-id latest --format ndjson",
        ],
        next_steps: &["telegram-agent-cli run --help", "telegram-agent-cli doctor --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli send",
        summary: "Send a text message to a resolved Telegram peer, optionally with a reply or inline keyboard.",
        prerequisites: &["Ensure the selected account exists and the target peer can be resolved."],
        examples: &[
            "telegram-agent-cli send --as alice --to qa-bot --text /start",
            "telegram-agent-cli send --as bot --to @user --text 'Choose:' --reply-keyboard '[\"Yes\",\"No\"]'",
            "telegram-agent-cli send --as bot --to @user --text 'Click:' --inline-keyboard '[\"OK:callback:done\"]'",
        ],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli message follow --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli send-file",
        summary: "Send a file to a resolved Telegram peer with an optional caption and keyboard.",
        prerequisites: &["Ensure the selected account exists, the peer resolves, and the file path is readable."],
        examples: &[
            "telegram-agent-cli send-file --as alice --to qa-bot fixtures/sample.txt --caption hello",
            "telegram-agent-cli send-file --as bot --to @user doc.pdf --inline-keyboard '[\"Accept:callback:yes\"]'",
        ],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli wait --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli wait",
        summary: "Wait for one matching incoming message from a target chat.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before waiting."],
        examples: &["telegram-agent-cli wait --as alice --chat qa-bot --text-contains Welcome --timeout 5s"],
        next_steps: &["telegram-agent-cli message follow --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli run",
        summary: "Execute a scenario file and persist run events for later export.",
        prerequisites: &["Ensure the scenario path exists and references accounts or peers that telegram-agent-cli can resolve."],
        examples: &["telegram-agent-cli run fixtures/scenarios/echo.yaml"],
        next_steps: &["telegram-agent-cli export --help", "telegram-agent-cli doctor --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli repl",
        summary: "Open an interactive chat REPL for sending messages, reading replies, and invoking slash commands.",
        prerequisites: &[
            "Ensure the selected account exists and can resolve the requested chat.",
            "Use a bot, alias, username, or peer ID that telegram-agent-cli can resolve before entering the REPL.",
        ],
        examples: &["telegram-agent-cli repl --as alice --chat qa-bot", "Inside the REPL, run /help"],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli send --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli mcp",
        summary: "Start telegram-agent-cli as a Model Context Protocol (MCP) server, exposing Telegram operations as tools for AI agents over stdio.",
        prerequisites: &[
            "Ensure at least one account is configured and logged in before starting the MCP server.",
        ],
        examples: &[
            "telegram-agent-cli mcp",
            "Configure in Claude Desktop: {\"command\": \"telegram-agent-cli\", \"args\": [\"mcp\"]}",
        ],
        next_steps: &["telegram-agent-cli account --help", "telegram-agent-cli repl --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli daemon start",
        summary: "Launch the managed background JSON-RPC daemon and wait until it is ready.",
        prerequisites: &[
            "Use this when you want the MCP-style tool server to keep running after the current terminal session exits.",
        ],
        examples: &[
            "telegram-agent-cli daemon start",
            "telegram-agent-cli daemon start --timeout 15s",
        ],
        next_steps: &["telegram-agent-cli daemon status --help", "telegram-agent-cli daemon stop --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli daemon stop",
        summary: "Request a graceful shutdown of the managed background daemon and wait for it to exit.",
        prerequisites: &["Ensure the daemon was previously started from the same runtime state root before stopping it."],
        examples: &[
            "telegram-agent-cli daemon stop",
            "telegram-agent-cli daemon stop --timeout 15s",
        ],
        next_steps: &["telegram-agent-cli daemon status --help", "telegram-agent-cli daemon start --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli daemon restart",
        summary: "Restart the managed background daemon when you need to recover from a stale or unhealthy state.",
        prerequisites: &["Use this when daemon status reports stale state or when you need to reload the background service from the CLI."],
        examples: &[
            "telegram-agent-cli daemon restart",
            "telegram-agent-cli daemon restart --timeout 20s",
        ],
        next_steps: &["telegram-agent-cli daemon status --help", "telegram-agent-cli daemon stop --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli daemon status",
        summary: "Inspect whether the managed background daemon is running, stale, or stopped.",
        prerequisites: &["Use this before starting or stopping the daemon if you need to confirm the current managed state."],
        examples: &[
            "telegram-agent-cli daemon status",
            "telegram-agent-cli daemon status --format json",
        ],
        next_steps: &["telegram-agent-cli daemon start --help", "telegram-agent-cli daemon restart --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account add-user",
        summary: "Register a Telegram user account with API credentials and optional phone number.",
        prerequisites: &["Ensure you have a valid Telegram API ID and API hash before adding the account."],
        examples: &["telegram-agent-cli account add-user --name alice --api-id 12345 --api-hash <hash> --phone +10000000000"],
        next_steps: &["telegram-agent-cli account login --help", "telegram-agent-cli account list --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account add-bot",
        summary: "Register a Telegram bot account with a bot token and optional API credentials.",
        prerequisites: &["Ensure the bot token is available directly or through an environment variable before running the command."],
        examples: &["telegram-agent-cli account add-bot --name bot --token-env TELEGRAM_CLI_BOT_TOKEN"],
        next_steps: &["telegram-agent-cli account login --help", "telegram-agent-cli account list --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account list",
        summary: "Inspect configured accounts, account kinds, and login state.",
        prerequisites: &["Add at least one user or bot account before expecting non-empty output."],
        examples: &["telegram-agent-cli account list"],
        next_steps: &["telegram-agent-cli account use --help", "telegram-agent-cli doctor --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account use",
        summary: "Set the default account used when later commands receive --as default.",
        prerequisites: &["Ensure the target account name already exists in local account storage."],
        examples: &["telegram-agent-cli account use alice"],
        next_steps: &["telegram-agent-cli account list --help", "telegram-agent-cli doctor --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account login",
        summary: "Authenticate a stored account by QR flow or credential-based login.",
        prerequisites: &["Ensure the account already exists before attempting login."],
        examples: &[
            "telegram-agent-cli account login alice --qr",
            "telegram-agent-cli account login alice --code-env TELEGRAM_CLI_LOGIN_CODE --password-env TELEGRAM_CLI_2FA_PASSWORD",
        ],
        next_steps: &["telegram-agent-cli account list --help", "telegram-agent-cli account logout --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli account logout",
        summary: "Clear the stored session for an account and mark it as logged out.",
        prerequisites: &["Ensure the account exists before attempting logout."],
        examples: &["telegram-agent-cli account logout alice"],
        next_steps: &["telegram-agent-cli account login --help", "telegram-agent-cli account list --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli alias set",
        summary: "Bind a memorable alias to a resolved Telegram peer for later commands.",
        prerequisites: &["Ensure the selected account exists and the query resolves to a real peer."],
        examples: &["telegram-agent-cli alias set qa-bot qa_bot"],
        next_steps: &["telegram-agent-cli alias list --help", "telegram-agent-cli peer resolve --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli alias list",
        summary: "Inspect saved alias to peer mappings.",
        prerequisites: &["Create at least one alias before expecting non-empty output."],
        examples: &["telegram-agent-cli alias list"],
        next_steps: &["telegram-agent-cli alias set --help", "telegram-agent-cli peer resolve --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli peer resolve",
        summary: "Resolve an alias, username, or numeric ID into a Telegram peer record.",
        prerequisites: &["Ensure the selected account exists when resolving usernames or aliases against Telegram state."],
        examples: &["telegram-agent-cli peer resolve --as alice qa-bot"],
        next_steps: &["telegram-agent-cli alias set --help", "telegram-agent-cli list contacts --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli list contacts",
        summary: "List direct-contact peers available to the selected account.",
        prerequisites: &["Ensure the selected account exists before listing contacts."],
        examples: &["telegram-agent-cli list contacts --as alice"],
        next_steps: &["telegram-agent-cli peer resolve --help", "telegram-agent-cli alias set --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli list chats",
        summary: "List groups and channels available to the selected account.",
        prerequisites: &["Ensure the selected account exists before listing chats."],
        examples: &["telegram-agent-cli list chats --as alice"],
        next_steps: &["telegram-agent-cli peer resolve --help", "telegram-agent-cli alias set --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message click-button",
        summary: "Click an inline button in a bot message and optionally wait for a response.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the target message/button is available."],
        examples: &["telegram-agent-cli message click-button --as alice --chat qa-bot start --wait-timeout 5s"],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli message wait --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message list-actions",
        summary: "List inline buttons, reply keyboard buttons, and bot command/menu actions available in a chat.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before discovering interactive actions."],
        examples: &[
            "telegram-agent-cli message list-actions --as alice --chat qa-bot",
            "telegram-agent-cli message list-actions --as alice --chat qa-bot --message-id 123",
        ],
        next_steps: &["telegram-agent-cli message trigger-action --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message trigger-action",
        summary: "Trigger an action previously discovered from inline buttons, reply keyboards, or bot command menus.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the target action can be discovered from telegram-agent-cli message list-actions."],
        examples: &[
            "telegram-agent-cli message trigger-action --as alice --chat qa-bot Start",
            "telegram-agent-cli message trigger-action --as alice --chat qa-bot bot-command:qa_bot:start --wait-timeout 5s",
        ],
        next_steps: &["telegram-agent-cli message list-actions --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message recv",
        summary: "Read recent messages from a Telegram chat with optional pagination and unread filtering.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before reading messages."],
        examples: &["telegram-agent-cli message recv --as alice --chat qa-bot --limit 20"],
        next_steps: &["telegram-agent-cli message follow --help", "telegram-agent-cli message unread --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message follow",
        summary: "Collect one or more future matching messages from a Telegram chat.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before following messages."],
        examples: &["telegram-agent-cli message follow --as alice --chat qa-bot --text-contains Welcome --timeout 20s --limit 2"],
        next_steps: &["telegram-agent-cli message wait --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message wait",
        summary: "Wait for one matching message within the message command family.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before waiting."],
        examples: &["telegram-agent-cli message wait --as alice --chat qa-bot --text-contains Welcome --timeout 20s"],
        next_steps: &["telegram-agent-cli message follow --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message unread",
        summary: "Inspect unread message counts and the latest unread content for a chat.",
        prerequisites: &["Ensure the selected account exists and the chat resolves before checking unread state."],
        examples: &["telegram-agent-cli message unread --as alice --chat qa-bot"],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli message wait --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message forward",
        summary: "Forward one or more messages from one chat to another.",
        prerequisites: &["Ensure the selected account exists and both source and destination chats resolve."],
        examples: &["telegram-agent-cli message forward --as alice --from group-a --to group-b --message-ids 123,124"],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli send --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message edit",
        summary: "Edit the text of an existing message sent by the current account.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the message was sent by this account."],
        examples: &["telegram-agent-cli message edit --as bot --chat @user --message-id 123 --text 'updated text'"],
        next_steps: &["telegram-agent-cli send --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message pin",
        summary: "Pin a message in a chat so it stays visible at the top.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the account has pin permission."],
        examples: &["telegram-agent-cli message pin --as alice --chat group-a --message-id 42"],
        next_steps: &["telegram-agent-cli message unpin --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message unpin",
        summary: "Unpin a pinned message in a chat.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the account has pin permission."],
        examples: &["telegram-agent-cli message unpin --as alice --chat group-a --message-id 42"],
        next_steps: &["telegram-agent-cli message pin --help", "telegram-agent-cli message recv --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli message download",
        summary: "Download media attached to a message to a local file path.",
        prerequisites: &["Ensure the selected account exists, the chat resolves, and the message contains downloadable media."],
        examples: &["telegram-agent-cli message download --as alice --chat qa-bot --message-id 123 --output photo.jpg"],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli send-file --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli send-photo",
        summary: "Send a photo to a resolved Telegram peer with an optional caption and keyboard.",
        prerequisites: &["Ensure the selected account exists, the peer resolves, and the image path is readable."],
        examples: &[
            "telegram-agent-cli send-photo --as alice --to qa-bot photo.jpg",
            "telegram-agent-cli send-photo --as bot --to @user image.png --caption 'Look!' --inline-keyboard '[\"Like:callback:1\"]'",
        ],
        next_steps: &["telegram-agent-cli message recv --help", "telegram-agent-cli send --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli bot set-commands",
        summary: "Set the bot command menu displayed in the chat when users type /.",
        prerequisites: &["Ensure the selected account is a logged-in bot with the appropriate permissions."],
        examples: &["telegram-agent-cli bot set-commands --as mybot --commands '/start|Start the bot,/help|Show help'"],
        next_steps: &["telegram-agent-cli bot set-info --help", "telegram-agent-cli account --help"],
    },
    LeafHelpMetadata {
        command_path: "telegram-agent-cli bot set-info",
        summary: "Set the bot description and about text visible on the bot profile.",
        prerequisites: &["Ensure the selected account is a logged-in bot with the appropriate permissions."],
        examples: &["telegram-agent-cli bot set-info --as mybot --description 'A helpful bot' --about 'v1.0'"],
        next_steps: &["telegram-agent-cli bot set-commands --help", "telegram-agent-cli account --help"],
    },
];

pub fn top_level_help_metadata() -> &'static TopLevelHelpMetadata {
    &TOP_LEVEL_HELP_METADATA
}

pub fn command_group_help_metadata(
    command_path: &str,
) -> Option<&'static CommandGroupHelpMetadata> {
    COMMAND_GROUP_HELP_METADATA
        .iter()
        .find(|metadata| metadata.command_path == command_path)
}

pub fn leaf_help_metadata(command_path: &str) -> Option<&'static LeafHelpMetadata> {
    LEAF_HELP_METADATA
        .iter()
        .find(|metadata| metadata.command_path == command_path)
}
