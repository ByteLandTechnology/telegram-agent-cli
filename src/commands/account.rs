use crate::app::AppContext;
use crate::cli::{
    AccountCommand, AccountListArgs, AccountLoginArgs, AccountLogoutArgs, AddBotArgs, AddUserArgs,
    UseAccountArgs,
};
use crate::errors::{Result, TelegramCliError};
use crate::output::contract::NextStep;
use crate::output::Format;
use crate::storage::{LoginState, NewAccount};
use crate::telegram::{LoginRequest, UserLoginRequest};

pub async fn run(context: &AppContext, command: AccountCommand, format: Format) -> Result<()> {
    match command {
        AccountCommand::AddUser(args) => add_user(context, args, format),
        AccountCommand::AddBot(args) => add_bot(context, args, format),
        AccountCommand::List(args) => list_accounts(context, args, format),
        AccountCommand::Use(args) => use_account(context, args, format),
        AccountCommand::Login(args) => login_account(context, args, format).await,
        AccountCommand::Logout(args) => logout_account(context, args, format).await,
    }
}

fn add_user(context: &AppContext, args: AddUserArgs, format: Format) -> Result<()> {
    let account_name = args.name.clone();
    let new_account = match args.phone {
        Some(phone) => NewAccount::user(args.name, args.api_id, args.api_hash, phone),
        None => NewAccount::user_qr(args.name, args.api_id, args.api_hash),
    };
    let account = context.repo.insert_account(new_account)?;
    format.print_result(
        "telegram-agent-cli account add-user",
        &format!("User account {} was added.", account.name),
        &serde_json::json!({
            "account": account.name,
            "login_state": "pending",
        }),
        vec![
            NextStep {
                action: "authorize_account".into(),
                command: format!("telegram-agent-cli account login {account_name}"),
            },
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
        ],
    )?;
    Ok(())
}

fn add_bot(context: &AppContext, args: AddBotArgs, format: Format) -> Result<()> {
    let account_name = args.name.clone();
    let token = match (args.token, args.token_env) {
        (Some(token), None) => token,
        (None, Some(env_name)) => std::env::var(&env_name).map_err(|_| {
            TelegramCliError::Message(format!("environment variable {env_name} is not set"))
        })?,
        (Some(_), Some(_)) => {
            return Err(TelegramCliError::Message(
                "use either --token or --token-env, not both".into(),
            ))
        }
        (None, None) => {
            return Err(TelegramCliError::Message(
                "either --token or --token-env is required".into(),
            ))
        }
    };
    let api_hash = resolve_secret_value(args.api_hash, args.api_hash_env, "API hash")?;

    let account =
        context
            .repo
            .insert_account(NewAccount::bot(args.name, token, args.api_id, api_hash))?;
    format.print_result(
        "telegram-agent-cli account add-bot",
        &format!("Bot account {} was added.", account.name),
        &serde_json::json!({
            "account": account.name,
            "login_state": "pending",
        }),
        vec![
            NextStep {
                action: "authorize_account".into(),
                command: format!("telegram-agent-cli account login {account_name}"),
            },
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
        ],
    )?;
    Ok(())
}

fn list_accounts(context: &AppContext, _args: AccountListArgs, format: Format) -> Result<()> {
    let accounts = context.repo.list_accounts()?;
    let total = accounts.len();
    let default_account = accounts
        .iter()
        .find(|account| account.is_default)
        .map(|account| account.name.clone());
    let output = context.attach_active_context(
        None,
        false,
        &AccountListOutput {
            accounts,
            total,
            default_account,
        },
    )?;

    format.print_result(
        "telegram-agent-cli account list",
        "Account list collected.",
        &output,
        vec![
            NextStep {
                action: "choose_default_account".into(),
                command: "telegram-agent-cli account use <name>".into(),
            },
            NextStep {
                action: "inspect_diagnostics".into(),
                command: "telegram-agent-cli doctor".into(),
            },
        ],
    )
}

fn use_account(context: &AppContext, args: UseAccountArgs, format: Format) -> Result<()> {
    context.repo.set_default_by_name(&args.name)?;
    let output = context.attach_active_context(
        None,
        false,
        &serde_json::json!({
            "default_account": args.name,
        }),
    )?;
    format.print_result(
        "telegram-agent-cli account use",
        &format!("Default account was set to {}.", args.name),
        &output,
        vec![
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
            NextStep {
                action: "inspect_diagnostics".into(),
                command: "telegram-agent-cli doctor".into(),
            },
        ],
    )?;
    Ok(())
}

async fn login_account(context: &AppContext, args: AccountLoginArgs, format: Format) -> Result<()> {
    let profile = context
        .repo
        .find_account_profile(&args.name)?
        .ok_or_else(|| TelegramCliError::Message(format!("account {} was not found", args.name)))?;

    let request = match profile.kind {
        crate::storage::AccountKind::Bot => {
            if args.qr {
                return Err(TelegramCliError::Message(
                    "QR login is not supported for bot accounts".into(),
                ));
            }
            LoginRequest::Bot
        }
        crate::storage::AccountKind::User => {
            if args.qr {
                LoginRequest::UserQr
            } else {
                LoginRequest::User(UserLoginRequest {
                    code: resolve_secret_value(args.code, args.code_env, "login code")?,
                    password: resolve_secret_value(args.password, args.password_env, "password")?,
                })
            }
        }
    };

    if args.session.is_some() || args.session_env.is_some() {
        return Err(TelegramCliError::Message(
            "session import is not supported by the live adapter; use code/password login instead"
                .into(),
        ));
    }

    context.telegram.login(&args.name, request).await?;
    context
        .repo
        .mark_login_state(&args.name, LoginState::Authorized)?;
    let output = context.attach_active_context(
        None,
        false,
        &serde_json::json!({
            "account": args.name,
            "login_state": "authorized",
        }),
    )?;
    format.print_result(
        "telegram-agent-cli account login",
        &format!("Account {} is now authorized.", args.name),
        &output,
        vec![
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
            NextStep {
                action: "inspect_diagnostics".into(),
                command: "telegram-agent-cli doctor".into(),
            },
        ],
    )?;
    Ok(())
}

async fn logout_account(
    context: &AppContext,
    args: AccountLogoutArgs,
    format: Format,
) -> Result<()> {
    context.telegram.logout(&args.name).await?;
    context.repo.clear_session(&args.name)?;
    let output = context.attach_active_context(
        None,
        false,
        &serde_json::json!({
            "account": args.name,
            "login_state": "logged_out",
        }),
    )?;
    format.print_result(
        "telegram-agent-cli account logout",
        &format!("Stored session for {} was cleared.", args.name),
        &output,
        vec![
            NextStep {
                action: "inspect_login_help".into(),
                command: "telegram-agent-cli account login --help".into(),
            },
            NextStep {
                action: "inspect_accounts".into(),
                command: "telegram-agent-cli account list".into(),
            },
        ],
    )?;
    Ok(())
}

fn resolve_secret_value(
    direct: Option<String>,
    env_name: Option<String>,
    label: &str,
) -> Result<Option<String>> {
    match (direct, env_name) {
        (Some(value), None) => Ok(Some(value)),
        (None, Some(env_name)) => std::env::var(&env_name).map(Some).map_err(|_| {
            TelegramCliError::Message(format!(
                "environment variable {env_name} is not set for {label}"
            ))
        }),
        (Some(_), Some(_)) => Err(TelegramCliError::Message(format!(
            "use either direct {label} input or an environment variable, not both"
        ))),
        (None, None) => Ok(None),
    }
}

#[derive(serde::Serialize)]
struct AccountListOutput {
    accounts: Vec<crate::storage::AccountRecord>,
    total: usize,
    default_account: Option<String>,
}
