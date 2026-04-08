use crate::errors::Result;
use crate::telegram::auth::LoginRequest;
use crate::telegram::keyboard::ReplyMarkupConfig;
use crate::telegram::list::{Chat, Contact};
use crate::telegram::messages::{
    ActionInvocationResult, IncomingMessage, InteractiveAction, InteractiveActionKind,
    InteractiveActionSource, SentMessage,
};
use crate::telegram::peers::ResolvedPeer;
use crate::telegram::updates::MessageFilter;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64URL;
use base64::Engine as _;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[async_trait(?Send)]
pub trait TelegramAdapter {
    async fn login(&self, account_name: &str, request: LoginRequest) -> Result<()>;

    async fn logout(&self, account_name: &str) -> Result<()>;

    async fn resolve_peer(&self, account_name: &str, query: &str) -> Result<ResolvedPeer>;

    async fn send_text(
        &self,
        account_name: &str,
        peer_id: i64,
        text: &str,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage>;

    async fn send_file(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage>;

    async fn send_photo(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage>;

    async fn forward_messages(
        &self,
        account_name: &str,
        from_peer_id: i64,
        to_peer_id: i64,
        message_ids: &[i32],
    ) -> Result<Vec<i64>>;

    async fn edit_message(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: i32,
        text: &str,
    ) -> Result<()>;

    async fn pin_message(&self, account_name: &str, peer_id: i64, message_id: i32) -> Result<()>;

    async fn unpin_message(&self, account_name: &str, peer_id: i64, message_id: i32) -> Result<()>;

    async fn download_media(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: i32,
        output_path: &Path,
    ) -> Result<bool>;

    async fn recent_messages(
        &self,
        account_name: &str,
        peer_id: i64,
        limit: usize,
        offset_id: Option<i64>,
        unread_only: bool,
    ) -> Result<Vec<IncomingMessage>>;

    async fn wait_for_message(
        &self,
        account_name: &str,
        peer_id: i64,
        filter: &MessageFilter,
        timeout: Duration,
    ) -> Result<IncomingMessage>;

    async fn delete_message(&self, account_name: &str, peer_id: i64, message_id: i64)
        -> Result<()>;

    async fn click_button(
        &self,
        account_name: &str,
        peer_id: i64,
        button: &str,
        message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<Option<IncomingMessage>>;

    async fn list_actions(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: Option<i64>,
        search_limit: usize,
    ) -> Result<Vec<InteractiveAction>>;

    async fn trigger_action(
        &self,
        account_name: &str,
        peer_id: i64,
        action: &str,
        message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<ActionInvocationResult>;

    async fn list_contacts(&self, account_name: &str) -> Result<Vec<Contact>>;

    async fn list_chats(&self, account_name: &str) -> Result<Vec<Chat>>;

    async fn set_bot_commands(
        &self,
        account_name: &str,
        commands: &[(String, String)],
    ) -> Result<()>;

    async fn set_bot_info(
        &self,
        account_name: &str,
        description: Option<&str>,
        about: Option<&str>,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
struct MockStoredMessage {
    message: IncomingMessage,
    actions: Vec<InteractiveAction>,
}

#[derive(Debug, Default)]
struct MockState {
    next_message_id: i64,
    resolved_peers: HashMap<String, ResolvedPeer>,
    sent_messages: Vec<SentMessage>,
    sent_peers: Vec<ResolvedPeer>,
    incoming_messages: HashMap<i64, VecDeque<IncomingMessage>>,
    message_history: HashMap<i64, VecDeque<MockStoredMessage>>,
    bot_actions: HashMap<i64, Vec<InteractiveAction>>,
    deleted_messages: Vec<(i64, i64)>,
    authorized_accounts: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct MockTelegramAdapter {
    state: Arc<Mutex<MockState>>,
}

impl MockTelegramAdapter {
    pub fn with_peer(alias: impl Into<String>, peer: ResolvedPeer) -> Self {
        let adapter = Self::default();
        adapter.register_peer(alias, peer);
        adapter
    }

    pub fn register_peer(&self, alias: impl Into<String>, peer: ResolvedPeer) {
        let mut state = self.state.lock().unwrap();
        state.resolved_peers.insert(alias.into(), peer);
    }

    pub fn queue_incoming(&self, message: IncomingMessage) {
        self.queue_interactive_message(message, Vec::new());
    }

    pub fn queue_incoming_inline_buttons(
        &self,
        peer_id: i64,
        text: impl Into<String>,
        buttons: &[(&str, &str)],
    ) {
        let message = self.build_incoming_text_message(peer_id, text);
        let actions = buttons
            .iter()
            .map(|(label, payload)| {
                if payload.starts_with("http://") || payload.starts_with("https://") {
                    inline_url_action(message.message_id, label, payload)
                } else {
                    inline_callback_action(message.message_id, label, payload)
                }
            })
            .collect();
        self.record_interactive_message(message, actions);
    }

    pub fn queue_incoming_reply_keyboard(
        &self,
        peer_id: i64,
        text: impl Into<String>,
        buttons: &[&str],
    ) {
        let message = self.build_incoming_text_message(peer_id, text);
        let actions = buttons
            .iter()
            .map(|label| reply_keyboard_text_action(message.message_id, label))
            .collect();
        self.record_interactive_message(message, actions);
    }

    pub fn register_bot_commands(
        &self,
        peer_id: i64,
        bot_username: impl Into<String>,
        commands: &[(&str, &str)],
    ) {
        let bot_username = bot_username.into();
        let mut state = self.state.lock().unwrap();
        let registered = state.bot_actions.entry(peer_id).or_default();
        registered.extend(
            commands.iter().map(|(command, description)| {
                bot_command_action(&bot_username, command, description)
            }),
        );
    }

    pub fn register_bot_menu_url(
        &self,
        peer_id: i64,
        bot_username: impl Into<String>,
        label: impl Into<String>,
        url: impl Into<String>,
    ) {
        let bot_username = bot_username.into();
        let label = label.into();
        let url = url.into();
        let mut state = self.state.lock().unwrap();
        state
            .bot_actions
            .entry(peer_id)
            .or_default()
            .push(bot_menu_url_action(&bot_username, &label, &url));
    }

    pub fn queue_incoming_text(&self, peer_id: i64, text: impl Into<String>) {
        self.queue_incoming(self.build_incoming_text_message(peer_id, text));
    }

    pub fn sent_messages(&self) -> Vec<SentMessage> {
        self.state.lock().unwrap().sent_messages.clone()
    }

    pub fn sent_peers(&self) -> Vec<ResolvedPeer> {
        self.state.lock().unwrap().sent_peers.clone()
    }

    pub fn deleted_messages(&self) -> Vec<(i64, i64)> {
        self.state.lock().unwrap().deleted_messages.clone()
    }

    fn build_incoming_text_message(
        &self,
        peer_id: i64,
        text: impl Into<String>,
    ) -> IncomingMessage {
        let text = text.into();
        let message_id = {
            let mut state = self.state.lock().unwrap();
            state.next_message_id += 1;
            state.next_message_id
        };
        IncomingMessage {
            message_id,
            peer_id,
            sender: Some("mock-bot".into()),
            text: Some(text),
            kind: crate::telegram::messages::MessageKind::Text,
            reply_to_message_id: None,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    fn queue_interactive_message(&self, message: IncomingMessage, actions: Vec<InteractiveAction>) {
        let mut state = self.state.lock().unwrap();
        let peer_id = message.peer_id;
        state
            .incoming_messages
            .entry(peer_id)
            .or_default()
            .push_back(message.clone());
        state
            .message_history
            .entry(peer_id)
            .or_default()
            .push_back(MockStoredMessage { message, actions });
    }

    fn record_interactive_message(
        &self,
        message: IncomingMessage,
        actions: Vec<InteractiveAction>,
    ) {
        let mut state = self.state.lock().unwrap();
        state
            .message_history
            .entry(message.peer_id)
            .or_default()
            .push_back(MockStoredMessage { message, actions });
    }
}

#[async_trait(?Send)]
impl TelegramAdapter for MockTelegramAdapter {
    async fn login(&self, account_name: &str, _request: LoginRequest) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state
            .authorized_accounts
            .iter()
            .any(|name| name == account_name)
        {
            state.authorized_accounts.push(account_name.to_string());
        }
        Ok(())
    }

    async fn logout(&self, account_name: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state
            .authorized_accounts
            .retain(|name| name != account_name);
        Ok(())
    }

    async fn resolve_peer(&self, _account_name: &str, query: &str) -> Result<ResolvedPeer> {
        let state = self.state.lock().unwrap();
        state
            .resolved_peers
            .get(query)
            .cloned()
            .or_else(|| {
                state
                    .resolved_peers
                    .get(query.trim_start_matches('@'))
                    .cloned()
            })
            .ok_or_else(|| {
                crate::errors::TelegramCliError::Message(format!("peer {query} was not found"))
            })
    }

    async fn send_text(
        &self,
        account_name: &str,
        peer_id: i64,
        text: &str,
        _reply_to: Option<i32>,
        _reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let mut state = self.state.lock().unwrap();
        state.next_message_id += 1;
        if let Some(peer) = state
            .resolved_peers
            .values()
            .find(|peer| peer.peer_id == peer_id)
            .cloned()
        {
            state.sent_peers.push(peer);
        }
        let message = SentMessage {
            message_id: state.next_message_id,
            account_name: account_name.to_string(),
            peer_id,
            text: Some(text.to_string()),
            file_path: None,
            caption: None,
            timestamp: Utc::now().to_rfc3339(),
        };
        state.sent_messages.push(message.clone());
        Ok(message)
    }

    async fn send_file(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        _reply_to: Option<i32>,
        _reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let mut state = self.state.lock().unwrap();
        state.next_message_id += 1;
        if let Some(peer) = state
            .resolved_peers
            .values()
            .find(|peer| peer.peer_id == peer_id)
            .cloned()
        {
            state.sent_peers.push(peer);
        }
        let message = SentMessage {
            message_id: state.next_message_id,
            account_name: account_name.to_string(),
            peer_id,
            text: None,
            file_path: Some(path.display().to_string()),
            caption: caption.map(ToOwned::to_owned),
            timestamp: Utc::now().to_rfc3339(),
        };
        state.sent_messages.push(message.clone());
        Ok(message)
    }

    async fn recent_messages(
        &self,
        _account_name: &str,
        peer_id: i64,
        limit: usize,
        _offset_id: Option<i64>,
        _unread_only: bool,
    ) -> Result<Vec<IncomingMessage>> {
        let state = self.state.lock().unwrap();
        let messages = state
            .incoming_messages
            .get(&peer_id)
            .map(|messages| messages.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default();
        Ok(messages)
    }

    async fn wait_for_message(
        &self,
        _account_name: &str,
        peer_id: i64,
        filter: &MessageFilter,
        timeout: Duration,
    ) -> Result<IncomingMessage> {
        let started_at = Instant::now();

        loop {
            {
                let mut state = self.state.lock().unwrap();
                if let Some(queue) = state.incoming_messages.get_mut(&peer_id) {
                    if let Some(index) = queue.iter().position(|message| filter.matches(message)) {
                        let message = queue.remove(index).ok_or_else(|| {
                            crate::errors::TelegramCliError::Message(
                                "failed to dequeue incoming message".into(),
                            )
                        })?;
                        return Ok(message);
                    }
                }
            }

            if started_at.elapsed() >= timeout {
                return Err(crate::errors::TelegramCliError::Message(format!(
                    "timed out waiting for message after {:?}",
                    timeout
                )));
            }

            sleep(Duration::from_millis(25)).await;
        }
    }

    async fn delete_message(
        &self,
        _account_name: &str,
        peer_id: i64,
        message_id: i64,
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.deleted_messages.push((peer_id, message_id));
        Ok(())
    }

    async fn send_photo(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        _reply_to: Option<i32>,
        _reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let mut state = self.state.lock().unwrap();
        state.next_message_id += 1;
        if let Some(peer) = state
            .resolved_peers
            .values()
            .find(|peer| peer.peer_id == peer_id)
            .cloned()
        {
            state.sent_peers.push(peer);
        }
        let message = SentMessage {
            message_id: state.next_message_id,
            account_name: account_name.to_string(),
            peer_id,
            text: caption.map(ToOwned::to_owned),
            file_path: Some(path.display().to_string()),
            caption: caption.map(ToOwned::to_owned),
            timestamp: Utc::now().to_rfc3339(),
        };
        state.sent_messages.push(message.clone());
        Ok(message)
    }

    async fn forward_messages(
        &self,
        _account_name: &str,
        _from_peer_id: i64,
        _to_peer_id: i64,
        message_ids: &[i32],
    ) -> Result<Vec<i64>> {
        let ids: Vec<i64> = message_ids.iter().map(|&id| id as i64).collect();
        Ok(ids)
    }

    async fn edit_message(
        &self,
        _account_name: &str,
        _peer_id: i64,
        _message_id: i32,
        _text: &str,
    ) -> Result<()> {
        Ok(())
    }

    async fn pin_message(
        &self,
        _account_name: &str,
        _peer_id: i64,
        _message_id: i32,
    ) -> Result<()> {
        Ok(())
    }

    async fn unpin_message(
        &self,
        _account_name: &str,
        _peer_id: i64,
        _message_id: i32,
    ) -> Result<()> {
        Ok(())
    }

    async fn download_media(
        &self,
        _account_name: &str,
        _peer_id: i64,
        _message_id: i32,
        _output_path: &Path,
    ) -> Result<bool> {
        Ok(false)
    }

    async fn click_button(
        &self,
        account_name: &str,
        peer_id: i64,
        _button: &str,
        _message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<Option<IncomingMessage>> {
        let filter = MessageFilter::default();
        match self
            .wait_for_message(account_name, peer_id, &filter, wait_timeout)
            .await
        {
            Ok(message) => Ok(Some(message)),
            Err(crate::errors::TelegramCliError::Message(message))
                if message.starts_with("timed out waiting for message after") =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    async fn list_actions(
        &self,
        _account_name: &str,
        peer_id: i64,
        message_id: Option<i64>,
        search_limit: usize,
    ) -> Result<Vec<InteractiveAction>> {
        let state = self.state.lock().unwrap();
        let mut actions = Vec::new();

        if let Some(history) = state.message_history.get(&peer_id) {
            let mut inspected_messages = 0usize;
            for stored in history.iter().rev() {
                if let Some(expected_message_id) = message_id {
                    if stored.message.message_id != expected_message_id {
                        continue;
                    }
                } else if inspected_messages >= search_limit {
                    break;
                }

                if message_id.is_none() {
                    inspected_messages += 1;
                }
                actions.extend(stored.actions.clone());
            }
        }

        actions.extend(state.bot_actions.get(&peer_id).cloned().unwrap_or_default());
        Ok(actions)
    }

    async fn trigger_action(
        &self,
        account_name: &str,
        peer_id: i64,
        action: &str,
        message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<ActionInvocationResult> {
        let actions = self
            .list_actions(account_name, peer_id, message_id, 50)
            .await?;
        let matched = resolve_action_match(&actions, action)?;

        if !matched.supported {
            return Err(crate::errors::TelegramCliError::Message(
                matched.unsupported_reason.clone().unwrap_or_else(|| {
                    format!(
                        "action `{}` is not supported by telegram-agent-cli",
                        matched.label
                    )
                }),
            ));
        }

        match matched.action_kind {
            InteractiveActionKind::InlineCallback => {
                let response =
                    wait_for_optional_response(self, account_name, peer_id, wait_timeout).await?;
                Ok(ActionInvocationResult {
                    action_id: matched.action_id.clone(),
                    action_kind: matched.action_kind,
                    label: matched.label.clone(),
                    effect: "callback_clicked".into(),
                    sent_text: None,
                    url: None,
                    response_received: response.is_some(),
                    response,
                })
            }
            InteractiveActionKind::ReplyKeyboardText | InteractiveActionKind::BotCommand => {
                let sent_text = matched
                    .trigger_text
                    .clone()
                    .unwrap_or_else(|| matched.label.clone());
                self.send_text(account_name, peer_id, &sent_text, None, None)
                    .await?;
                let response =
                    wait_for_optional_response(self, account_name, peer_id, wait_timeout).await?;
                Ok(ActionInvocationResult {
                    action_id: matched.action_id.clone(),
                    action_kind: matched.action_kind,
                    label: matched.label.clone(),
                    effect: "text_sent".into(),
                    sent_text: Some(sent_text),
                    url: None,
                    response_received: response.is_some(),
                    response,
                })
            }
            InteractiveActionKind::InlineUrl | InteractiveActionKind::BotMenuUrl => {
                Ok(ActionInvocationResult {
                    action_id: matched.action_id.clone(),
                    action_kind: matched.action_kind,
                    label: matched.label.clone(),
                    effect: "url_resolved".into(),
                    sent_text: None,
                    url: matched.url.clone(),
                    response_received: false,
                    response: None,
                })
            }
            _ => Err(crate::errors::TelegramCliError::Message(format!(
                "action `{}` is not currently triggerable by telegram-agent-cli",
                matched.label
            ))),
        }
    }

    async fn list_contacts(&self, _account_name: &str) -> Result<Vec<Contact>> {
        Ok(vec![])
    }

    async fn list_chats(&self, _account_name: &str) -> Result<Vec<Chat>> {
        Ok(vec![])
    }

    async fn set_bot_commands(
        &self,
        _account_name: &str,
        _commands: &[(String, String)],
    ) -> Result<()> {
        Ok(())
    }

    async fn set_bot_info(
        &self,
        _account_name: &str,
        _description: Option<&str>,
        _about: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }
}

fn inline_callback_action(message_id: i64, label: &str, callback_data: &str) -> InteractiveAction {
    let callback_data_base64 = BASE64URL.encode(callback_data.as_bytes());
    InteractiveAction {
        action_id: format!("inline-callback:{message_id}:{callback_data_base64}"),
        action_kind: InteractiveActionKind::InlineCallback,
        source: InteractiveActionSource::MessageMarkup,
        label: label.to_string(),
        description: None,
        message_id: Some(message_id),
        bot_username: None,
        callback_data_base64: Some(callback_data_base64),
        command: None,
        trigger_text: None,
        url: None,
        supported: true,
        unsupported_reason: None,
    }
}

fn inline_url_action(message_id: i64, label: &str, url: &str) -> InteractiveAction {
    InteractiveAction {
        action_id: format!(
            "inline-url:{message_id}:{}",
            BASE64URL.encode(url.as_bytes())
        ),
        action_kind: InteractiveActionKind::InlineUrl,
        source: InteractiveActionSource::MessageMarkup,
        label: label.to_string(),
        description: None,
        message_id: Some(message_id),
        bot_username: None,
        callback_data_base64: None,
        command: None,
        trigger_text: None,
        url: Some(url.to_string()),
        supported: true,
        unsupported_reason: None,
    }
}

fn reply_keyboard_text_action(message_id: i64, label: &str) -> InteractiveAction {
    InteractiveAction {
        action_id: format!(
            "reply-text:{message_id}:{}",
            BASE64URL.encode(label.as_bytes())
        ),
        action_kind: InteractiveActionKind::ReplyKeyboardText,
        source: InteractiveActionSource::MessageMarkup,
        label: label.to_string(),
        description: None,
        message_id: Some(message_id),
        bot_username: None,
        callback_data_base64: None,
        command: None,
        trigger_text: Some(label.to_string()),
        url: None,
        supported: true,
        unsupported_reason: None,
    }
}

fn bot_command_action(bot_username: &str, command: &str, description: &str) -> InteractiveAction {
    let normalized = normalize_command(command);
    InteractiveAction {
        action_id: format!(
            "bot-command:{bot_username}:{}",
            normalized.trim_start_matches('/')
        ),
        action_kind: InteractiveActionKind::BotCommand,
        source: InteractiveActionSource::BotProfile,
        label: normalized.clone(),
        description: Some(description.to_string()),
        message_id: None,
        bot_username: Some(bot_username.to_string()),
        callback_data_base64: None,
        command: Some(normalized.clone()),
        trigger_text: Some(normalized),
        url: None,
        supported: true,
        unsupported_reason: None,
    }
}

fn bot_menu_url_action(bot_username: &str, label: &str, url: &str) -> InteractiveAction {
    InteractiveAction {
        action_id: format!(
            "bot-menu-url:{bot_username}:{}",
            BASE64URL.encode(url.as_bytes())
        ),
        action_kind: InteractiveActionKind::BotMenuUrl,
        source: InteractiveActionSource::BotProfile,
        label: label.to_string(),
        description: None,
        message_id: None,
        bot_username: Some(bot_username.to_string()),
        callback_data_base64: None,
        command: None,
        trigger_text: None,
        url: Some(url.to_string()),
        supported: true,
        unsupported_reason: None,
    }
}

fn normalize_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn resolve_action_match<'a>(
    actions: &'a [InteractiveAction],
    query: &str,
) -> Result<&'a InteractiveAction> {
    let query = query.trim();
    let matches: Vec<&InteractiveAction> = actions
        .iter()
        .filter(|action| action_matches(action, query))
        .collect();

    match matches.len() {
        0 => Err(crate::errors::TelegramCliError::Message(format!(
            "action `{query}` was not found; run `telegram-agent-cli message list-actions --chat <peer>` to inspect available actions"
        ))),
        1 => Ok(matches[0]),
        _ => Err(crate::errors::TelegramCliError::Message(format!(
            "action `{query}` is ambiguous; rerun `telegram-agent-cli message list-actions --chat <peer>` and use the exact action_id"
        ))),
    }
}

fn action_matches(action: &InteractiveAction, query: &str) -> bool {
    if action.action_id == query || action.label == query {
        return true;
    }

    if let Some(callback_data_base64) = &action.callback_data_base64 {
        if callback_data_base64 == query {
            return true;
        }
    }

    if let Some(command) = &action.command {
        if command == query || command.trim_start_matches('/') == query.trim_start_matches('/') {
            return true;
        }
    }

    if let Some(trigger_text) = &action.trigger_text {
        if trigger_text == query {
            return true;
        }
    }

    action.url.as_deref() == Some(query)
}

async fn wait_for_optional_response(
    adapter: &MockTelegramAdapter,
    account_name: &str,
    peer_id: i64,
    wait_timeout: Duration,
) -> Result<Option<IncomingMessage>> {
    let filter = MessageFilter::default();
    match adapter
        .wait_for_message(account_name, peer_id, &filter, wait_timeout)
        .await
    {
        Ok(message) => Ok(Some(message)),
        Err(crate::errors::TelegramCliError::Message(message))
            if message.starts_with("timed out waiting for message after") =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
