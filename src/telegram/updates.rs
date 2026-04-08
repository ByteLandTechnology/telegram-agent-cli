use crate::telegram::messages::{IncomingMessage, MessageKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFilter {
    pub sender: Option<String>,
    pub text_equals: Option<String>,
    pub text_contains: Option<String>,
    pub kind: Option<MessageKind>,
    pub reply_to_message_id: Option<i64>,
}

impl MessageFilter {
    pub fn matches(&self, message: &IncomingMessage) -> bool {
        if let Some(sender) = &self.sender {
            if message.sender.as_deref() != Some(sender.as_str()) {
                return false;
            }
        }

        if let Some(expected) = &self.text_equals {
            if message.text.as_deref() != Some(expected.as_str()) {
                return false;
            }
        }

        if let Some(fragment) = &self.text_contains {
            if !message
                .text
                .as_deref()
                .is_some_and(|text| text.contains(fragment))
            {
                return false;
            }
        }

        if let Some(kind) = self.kind {
            if message.kind != kind {
                return false;
            }
        }

        if let Some(reply_to) = self.reply_to_message_id {
            if message.reply_to_message_id != Some(reply_to) {
                return false;
            }
        }

        true
    }
}
