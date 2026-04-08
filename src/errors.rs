pub type Result<T> = std::result::Result<T, TelegramCliError>;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum TelegramCliError {
    #[error("{0}")]
    Message(String),

    #[error("{message}")]
    CliUsage {
        command_path: String,
        help_command: String,
        message: String,
        rendered: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

impl TelegramCliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Message(_) => "message_error",
            Self::CliUsage { .. } => "cli_usage_error",
            Self::Io(_) => "io_error",
            Self::Database(_) => "database_error",
        }
    }

    pub fn rendered(&self) -> Option<&str> {
        match self {
            Self::CliUsage { rendered, .. } => Some(rendered),
            _ => None,
        }
    }

    pub fn command_path(&self) -> Option<&str> {
        match self {
            Self::CliUsage { command_path, .. } => Some(command_path),
            _ => None,
        }
    }

    pub fn help_command(&self) -> Option<&str> {
        match self {
            Self::CliUsage { help_command, .. } => Some(help_command),
            _ => None,
        }
    }
}
