use crate::app::AppContext;
use crate::cli::{ChatListArgs, ContactListArgs, ListCommand};
use crate::errors::Result;
use crate::output::contract::NextStep;
use crate::output::Format;

pub async fn run(context: &AppContext, command: ListCommand, format: Format) -> Result<()> {
    match command {
        ListCommand::Contacts(args) => list_contacts(context, args, format).await,
        ListCommand::Chats(args) => list_chats(context, args, format).await,
    }
}

async fn list_contacts(context: &AppContext, args: ContactListArgs, format: Format) -> Result<()> {
    let account_name = context.resolve_account_name(&args.as_account)?;
    let contacts = context.telegram.list_contacts(&account_name).await?;
    let total = contacts.len();

    let output = context.attach_active_context(
        Some(&args.as_account),
        true,
        &ContactsOutput { contacts, total },
    )?;
    format.print_result(
        "telegram-agent-cli list contacts",
        "Contact list collected.",
        &output,
        vec![NextStep {
            action: "resolve_peer".into(),
            command: "telegram-agent-cli peer resolve <query>".into(),
        }],
    )
}

async fn list_chats(context: &AppContext, args: ChatListArgs, format: Format) -> Result<()> {
    let account_name = context.resolve_account_name(&args.as_account)?;
    let chats = context.telegram.list_chats(&account_name).await?;
    let total = chats.len();

    let output = context.attach_active_context(
        Some(&args.as_account),
        true,
        &ChatsOutput { chats, total },
    )?;
    format.print_result(
        "telegram-agent-cli list chats",
        "Chat list collected.",
        &output,
        vec![NextStep {
            action: "set_alias".into(),
            command: "telegram-agent-cli alias set <alias> <query>".into(),
        }],
    )
}

#[derive(serde::Serialize)]
struct ContactsOutput {
    contacts: Vec<crate::telegram::Contact>,
    total: usize,
}

#[derive(serde::Serialize)]
struct ChatsOutput {
    chats: Vec<crate::telegram::Chat>,
    total: usize,
}
