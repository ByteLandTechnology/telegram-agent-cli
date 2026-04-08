use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    User,
    Bot,
}

impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Bot => "bot",
        }
    }
}

impl FromStr for AccountKind {
    type Err = crate::errors::TelegramCliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "bot" => Ok(Self::Bot),
            other => Err(crate::errors::TelegramCliError::Message(format!(
                "unsupported account kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginState {
    Pending,
    Authorized,
}

impl LoginState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Authorized => "authorized",
        }
    }
}

impl FromStr for LoginState {
    type Err = crate::errors::TelegramCliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "authorized" => Ok(Self::Authorized),
            other => Err(crate::errors::TelegramCliError::Message(format!(
                "unsupported login state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAccount {
    pub name: String,
    pub kind: AccountKind,
    pub api_id: Option<i32>,
    pub api_hash: Option<String>,
    pub phone: Option<String>,
    pub bot_token: Option<String>,
    pub login_state: LoginState,
}

impl NewAccount {
    pub fn user(
        name: impl Into<String>,
        api_id: i32,
        api_hash: impl Into<String>,
        phone: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: AccountKind::User,
            api_id: Some(api_id),
            api_hash: Some(api_hash.into()),
            phone: Some(phone.into()),
            bot_token: None,
            login_state: LoginState::Pending,
        }
    }

    pub fn user_qr(name: impl Into<String>, api_id: i32, api_hash: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: AccountKind::User,
            api_id: Some(api_id),
            api_hash: Some(api_hash.into()),
            phone: None,
            bot_token: None,
            login_state: LoginState::Pending,
        }
    }

    pub fn bot(
        name: impl Into<String>,
        bot_token: impl Into<String>,
        api_id: Option<i32>,
        api_hash: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: AccountKind::Bot,
            api_id,
            api_hash,
            phone: None,
            bot_token: Some(bot_token.into()),
            login_state: LoginState::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: i64,
    pub name: String,
    pub kind: AccountKind,
    pub login_state: LoginState,
    pub is_default: bool,
    pub api_id: Option<i32>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountProfile {
    pub id: i64,
    pub name: String,
    pub kind: AccountKind,
    pub login_state: LoginState,
    pub is_default: bool,
    pub api_id: Option<i32>,
    pub api_hash: Option<String>,
    pub phone: Option<String>,
    pub bot_token: Option<String>,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasRecord {
    pub alias: String,
    pub peer_id: i64,
    pub peer_kind: crate::telegram::PeerKind,
    pub display_name: String,
    pub username: Option<String>,
    pub packed_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunRecord {
    pub id: i64,
    pub scenario_path: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEventRecord {
    pub id: i64,
    pub run_id: i64,
    pub step_name: String,
    pub payload_json: String,
    pub created_at: String,
}
