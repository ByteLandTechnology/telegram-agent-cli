use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Text,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveActionKind {
    InlineCallback,
    InlineUrl,
    ReplyKeyboardText,
    ReplyKeyboardRequestPhone,
    ReplyKeyboardRequestGeo,
    ReplyKeyboardRequestPoll,
    ReplyKeyboardRequestPeer,
    BotCommand,
    BotMenuUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveActionSource {
    MessageMarkup,
    BotProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractiveAction {
    pub action_id: String,
    pub action_kind: InteractiveActionKind,
    pub source: InteractiveActionSource,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInvocationResult {
    pub action_id: String,
    pub action_kind: InteractiveActionKind,
    pub label: String,
    pub effect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub response_received: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<IncomingMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentMessage {
    pub message_id: i64,
    pub account_name: String,
    pub peer_id: i64,
    pub text: Option<String>,
    pub file_path: Option<String>,
    pub caption: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub message_id: i64,
    pub peer_id: i64,
    pub sender: Option<String>,
    pub text: Option<String>,
    pub kind: MessageKind,
    pub reply_to_message_id: Option<i64>,
    pub timestamp: String,
}
