use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerKind {
    User,
    Bot,
    Group,
    Channel,
}

impl PeerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Bot => "bot",
            Self::Group => "group",
            Self::Channel => "channel",
        }
    }
}

impl FromStr for PeerKind {
    type Err = crate::errors::TelegramCliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "bot" => Ok(Self::Bot),
            "group" => Ok(Self::Group),
            "channel" => Ok(Self::Channel),
            other => Err(crate::errors::TelegramCliError::Message(format!(
                "unsupported peer kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPeer {
    pub peer_id: i64,
    pub peer_kind: PeerKind,
    pub display_name: String,
    pub username: Option<String>,
    pub packed_hex: Option<String>,
}
