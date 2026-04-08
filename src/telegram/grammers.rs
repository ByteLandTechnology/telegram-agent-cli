use crate::config::paths::AppPaths;
use crate::config::settings::{ENV_API_HASH, ENV_API_ID};
use crate::errors::{Result, TelegramCliError};
use crate::output::guidance;
use crate::storage::{AccountProfile, AccountRepository, SecretStore};
use crate::telegram::adapter::TelegramAdapter;
use crate::telegram::auth::LoginRequest;
use crate::telegram::keyboard::ReplyMarkupConfig;
use crate::telegram::messages::{
    ActionInvocationResult, IncomingMessage, InteractiveAction, InteractiveActionKind,
    InteractiveActionSource, MessageKind, SentMessage,
};
use crate::telegram::peers::{PeerKind, ResolvedPeer};
use crate::telegram::updates::MessageFilter;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64URL;
use base64::Engine as _;
use grammers_client::client::UpdatesConfiguration;
use grammers_client::message::InputMessage;
use grammers_client::peer::Peer;
use grammers_client::session::types::{
    ChannelKind as SessionChannelKind, ChannelState, DcOption, PeerAuth, PeerId, PeerInfo,
    PeerKind as SessionPeerKind, PeerRef, UpdatesState,
};
use grammers_client::session::updates::UpdatesLike;
use grammers_client::session::Session as _;
use grammers_client::session::SessionData;
use grammers_client::update::Update;
use grammers_client::{Client, SenderPool, SignInError};
use grammers_tl_types as tl;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct GrammersAdapter {
    paths: AppPaths,
    secrets: SecretStore,
}

struct ConnectedClient {
    client: Client,
    session: Arc<PersistedSessionStore>,
    handle: grammers_client::sender::SenderPoolFatHandle,
    updates: Option<tokio::sync::mpsc::UnboundedReceiver<UpdatesLike>>,
    pool_task: JoinHandle<()>,
    encrypted_session_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiCredentials {
    api_id: i32,
    api_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedSessionState {
    home_dc: i32,
    dc_options: HashMap<i32, DcOption>,
    #[serde(with = "peer_info_cache")]
    peer_infos: HashMap<PeerId, PeerInfo>,
    updates_state: UpdatesState,
}

impl Default for PersistedSessionState {
    fn default() -> Self {
        let data = SessionData::default();
        Self {
            home_dc: data.home_dc,
            dc_options: data.dc_options,
            peer_infos: data.peer_infos,
            updates_state: data.updates_state,
        }
    }
}

struct PersistedSessionStore(Mutex<PersistedSessionState>);

type SessionFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

mod peer_info_cache {
    use super::{PeerId, PeerInfo};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        peer_infos: &HashMap<PeerId, PeerInfo>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut entries: Vec<PeerInfo> = peer_infos.values().cloned().collect();
        entries.sort_by_key(|info| info.id().bot_api_dialog_id());
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<PeerId, PeerInfo>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<PeerInfo>::deserialize(deserializer)?;
        Ok(entries.into_iter().map(|info| (info.id(), info)).collect())
    }
}

impl PersistedSessionStore {
    fn from_state(state: PersistedSessionState) -> Self {
        Self(Mutex::new(state))
    }

    fn snapshot(&self) -> PersistedSessionState {
        self.0.lock().unwrap().clone()
    }
}

impl grammers_client::session::Session for PersistedSessionStore {
    fn home_dc_id(&self) -> i32 {
        self.0.lock().unwrap().home_dc
    }

    fn set_home_dc_id(&self, dc_id: i32) -> SessionFuture<'_, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().home_dc = dc_id;
        })
    }

    fn dc_option(&self, dc_id: i32) -> Option<DcOption> {
        self.0.lock().unwrap().dc_options.get(&dc_id).cloned()
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> SessionFuture<'_, ()> {
        let dc_option = dc_option.clone();
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .dc_options
                .insert(dc_option.id, dc_option);
        })
    }

    fn peer(&self, peer: PeerId) -> SessionFuture<'_, Option<PeerInfo>> {
        Box::pin(async move {
            let state = self.0.lock().unwrap();
            if peer.kind() == SessionPeerKind::UserSelf {
                return state
                    .peer_infos
                    .values()
                    .find(|info| {
                        matches!(
                            info,
                            PeerInfo::User {
                                is_self: Some(true),
                                ..
                            }
                        )
                    })
                    .cloned();
            }

            state.peer_infos.get(&peer).cloned()
        })
    }

    fn cache_peer(&self, peer: &PeerInfo) -> SessionFuture<'_, ()> {
        let peer = peer.clone();
        Box::pin(async move {
            self.0.lock().unwrap().peer_infos.insert(peer.id(), peer);
        })
    }

    fn updates_state(&self) -> SessionFuture<'_, UpdatesState> {
        Box::pin(async move { self.0.lock().unwrap().updates_state.clone() })
    }

    fn set_update_state(
        &self,
        update: grammers_client::session::types::UpdateState,
    ) -> SessionFuture<'_, ()> {
        Box::pin(async move {
            let mut data = self.0.lock().unwrap();

            match update {
                grammers_client::session::types::UpdateState::All(updates_state) => {
                    data.updates_state = updates_state;
                }
                grammers_client::session::types::UpdateState::Primary { pts, date, seq } => {
                    data.updates_state.pts = pts;
                    data.updates_state.date = date;
                    data.updates_state.seq = seq;
                }
                grammers_client::session::types::UpdateState::Secondary { qts } => {
                    data.updates_state.qts = qts;
                }
                grammers_client::session::types::UpdateState::Channel { id, pts } => {
                    data.updates_state
                        .channels
                        .retain(|channel| channel.id != id);
                    data.updates_state.channels.push(ChannelState { id, pts });
                }
            }
        })
    }
}

impl GrammersAdapter {
    pub fn new(paths: AppPaths, secrets: SecretStore) -> Self {
        Self { paths, secrets }
    }

    fn repo(&self) -> Result<AccountRepository> {
        AccountRepository::open(&self.paths.db_path, self.secrets.clone())
    }

    fn load_profile(&self, account_name: &str) -> Result<AccountProfile> {
        self.repo()?
            .find_account_profile(account_name)?
            .ok_or_else(|| {
                TelegramCliError::Message(format!("account {account_name} was not found"))
            })
    }

    fn runtime_session_path(&self, account_name: &str) -> Result<PathBuf> {
        let directory = self.paths.state_dir.join("sessions");
        std::fs::create_dir_all(&directory)?;
        Ok(directory.join(format!("{}.sqlite3", sanitize_account_name(account_name))))
    }

    fn encrypted_session_path(&self, account_name: &str) -> Result<PathBuf> {
        Ok(self
            .runtime_session_path(account_name)?
            .with_extension("sqlite3.enc"))
    }

    async fn connect_client(
        &self,
        account_name: &str,
    ) -> Result<(ConnectedClient, AccountProfile)> {
        let profile = self.load_profile(account_name)?;
        let api_id_env = std::env::var(ENV_API_ID).ok();
        let api_hash_env = std::env::var(ENV_API_HASH).ok();
        let credentials =
            resolve_api_credentials(&profile, api_id_env.as_deref(), api_hash_env.as_deref())?;
        let encrypted_session_path = self.encrypted_session_path(account_name)?;
        let session = Arc::new(load_encrypted_session_snapshot(
            &self.secrets,
            &encrypted_session_path,
        )?);

        let SenderPool {
            runner,
            updates,
            handle,
        } = SenderPool::new(Arc::clone(&session), credentials.api_id);
        let client = Client::new(handle.clone());
        let pool_task = tokio::spawn(runner.run());

        Ok((
            ConnectedClient {
                client,
                session,
                handle,
                updates: Some(updates),
                pool_task,
                encrypted_session_path,
            },
            profile,
        ))
    }

    async fn connect_authorized_client(
        &self,
        account_name: &str,
    ) -> Result<(ConnectedClient, AccountProfile)> {
        let (connected, profile) = self.connect_client(account_name).await?;
        let is_authorized = connected
            .client
            .is_authorized()
            .await
            .map_err(invocation_error)?;
        if is_authorized {
            Ok((connected, profile))
        } else {
            connected.shutdown(&self.secrets).await?;
            Err(TelegramCliError::Message(format!(
                "account {account_name} is not logged in; run `telegram-agent-cli account login {account_name}` first"
            )))
        }
    }

    async fn peer_ref_for_dialog_id(
        connected: &ConnectedClient,
        dialog_id: i64,
    ) -> Result<PeerRef> {
        let peer_id = peer_id_from_dialog_id(dialog_id)?;
        connected.session.peer_ref(peer_id).await.ok_or_else(|| {
            TelegramCliError::Message(format!(
                "peer {dialog_id} is not cached in the Telegram session; resolve it again before sending"
            ))
        })
    }

    fn map_peer(peer: &Peer) -> ResolvedPeer {
        let peer_kind = match peer {
            Peer::User(user) if user.is_bot() => PeerKind::Bot,
            Peer::User(_) => PeerKind::User,
            Peer::Group(_) => PeerKind::Group,
            Peer::Channel(_) => PeerKind::Channel,
        };

        ResolvedPeer {
            peer_id: peer.id().bot_api_dialog_id(),
            peer_kind,
            display_name: peer
                .name()
                .map(ToOwned::to_owned)
                .or_else(|| peer.username().map(ToOwned::to_owned))
                .unwrap_or_else(|| peer.id().bot_api_dialog_id().to_string()),
            username: peer.username().map(ToOwned::to_owned),
            packed_hex: None,
        }
    }
}

impl ConnectedClient {
    fn take_updates(&mut self) -> Result<tokio::sync::mpsc::UnboundedReceiver<UpdatesLike>> {
        self.updates.take().ok_or_else(|| {
            TelegramCliError::Message("Telegram updates stream is unavailable".into())
        })
    }

    async fn shutdown(mut self, secrets: &SecretStore) -> Result<()> {
        let encrypted_session_path = self.encrypted_session_path.clone();
        self.updates.take();
        self.handle.quit();
        let _ = self.pool_task.await;
        drop(self.client);
        persist_encrypted_session_snapshot(secrets, &self.session, &encrypted_session_path)
    }
}

async fn export_qr_login_token(
    connected: &ConnectedClient,
    credentials: &ApiCredentials,
) -> Result<tl::enums::auth::LoginToken> {
    let result = connected
        .client
        .invoke(&tl::functions::auth::ExportLoginToken {
            api_id: credentials.api_id,
            api_hash: credentials.api_hash.clone(),
            except_ids: vec![],
        })
        .await
        .map_err(invocation_error)?;

    resolve_qr_login_token_migration(connected, result).await
}

async fn resolve_qr_login_token_migration(
    connected: &ConnectedClient,
    mut result: tl::enums::auth::LoginToken,
) -> Result<tl::enums::auth::LoginToken> {
    for _ in 0..4 {
        let tl::enums::auth::LoginToken::MigrateTo(migrate) = result else {
            return Ok(result);
        };

        let old_dc_id = connected.session.home_dc_id();
        let new_dc_id = migrate.dc_id;

        if old_dc_id != new_dc_id {
            connected.handle.disconnect_from_dc(old_dc_id);
            connected.session.set_home_dc_id(new_dc_id).await;
        }

        result = connected
            .client
            .invoke(&tl::functions::auth::ImportLoginToken {
                token: migrate.token,
            })
            .await
            .map_err(invocation_error)?;
    }

    Err(TelegramCliError::Message(
        "QR login hit repeated DC migrations; retry the login command".into(),
    ))
}

#[async_trait(?Send)]
impl TelegramAdapter for GrammersAdapter {
    async fn login(&self, account_name: &str, request: LoginRequest) -> Result<()> {
        let (connected, profile) = self.connect_client(account_name).await?;
        let authorized = connected
            .client
            .is_authorized()
            .await
            .map_err(invocation_error)?;

        let outcome = async {
            if authorized {
                return Ok(());
            }

            let api_id_env = std::env::var(ENV_API_ID).ok();
            let api_hash_env = std::env::var(ENV_API_HASH).ok();
            let credentials =
                resolve_api_credentials(&profile, api_id_env.as_deref(), api_hash_env.as_deref())?;

            match request {
                LoginRequest::Bot => {
                    let token = profile.bot_token.as_deref().ok_or_else(|| {
                        TelegramCliError::Message(format!(
                            "bot account {account_name} is missing a stored token"
                        ))
                    })?;
                    connected
                        .client
                        .bot_sign_in(token, &credentials.api_hash)
                        .await
                        .map_err(invocation_error)?;
                }
                LoginRequest::User(request) => {
                    let phone = profile.phone.as_deref().ok_or_else(|| {
                        TelegramCliError::Message(format!(
                            "user account {account_name} is missing a stored phone number"
                        ))
                    })?;

                    let login_token = connected
                        .client
                        .request_login_code(phone, &credentials.api_hash)
                        .await
                        .map_err(invocation_error)?;

                    let code_owned;
                    let code = match request.code.as_deref() {
                        Some(c) => c,
                        None => {
                            code_owned = prompt_line("Enter the Telegram login code: ")
                                .map_err(|e| TelegramCliError::Message(e.to_string()))?;
                            &code_owned
                        }
                    };

                    match connected.client.sign_in(&login_token, code).await {
                        Ok(_) => {}
                        Err(SignInError::PasswordRequired(password_token)) => {
                            let password = request.password.as_deref().ok_or_else(|| {
                                TelegramCliError::Message(
                                    "this account requires 2FA; pass --password or --password-env"
                                        .into(),
                                )
                            })?;
                            connected
                                .client
                                .check_password(password_token, password)
                                .await
                                .map_err(sign_in_error)?;
                        }
                        Err(error) => return Err(sign_in_error(error)),
                    }
                }
                LoginRequest::UserQr => {
                    let result = export_qr_login_token(&connected, &credentials).await?;

                    let token = match result {
                        tl::enums::auth::LoginToken::Token(t) => t,
                        tl::enums::auth::LoginToken::Success(_) => return Ok(()),
                        tl::enums::auth::LoginToken::MigrateTo(_) => unreachable!(),
                    };

                    let url = format!("tg://login?token={}", BASE64URL.encode(&token.token));
                    println!(
                        "{}",
                        guidance::runtime_success(
                            "telegram-agent-cli account login",
                            format!(
                                "QR login is ready. Scan the terminal QR code with the Telegram app on your phone. The current code expires in about {} seconds.",
                                expiry_secs(token.expires)
                            ),
                            &[
                                "Keep this terminal open while Telegram scans and confirms the session.",
                                "If the QR code expires, run telegram-agent-cli account login <name> --qr again.",
                            ],
                        )
                    );
                    render_qr_to_terminal(&url)?;

                    let expires_at = compute_qr_expiry(token.expires);
                    loop {
                        tokio::time::sleep(Duration::from_secs(1)).await;

                        if tokio::time::Instant::now() >= expires_at {
                            return Err(TelegramCliError::Message(
                                "QR code expired; run `telegram-agent-cli account login --qr` again".into(),
                            ));
                        }

                        let result = export_qr_login_token(&connected, &credentials).await?;

                        match result {
                            tl::enums::auth::LoginToken::Success(_) => {
                                println!(
                                    "{}",
                                    guidance::runtime_success(
                                        "telegram-agent-cli account login",
                                        "QR login was confirmed.",
                                        &[
                                            "Run telegram-agent-cli account list to confirm login state.",
                                            "Run telegram-agent-cli doctor to inspect current diagnostics.",
                                        ],
                                    )
                                );
                                return Ok(());
                            }
                            tl::enums::auth::LoginToken::Token(_) => continue,
                            tl::enums::auth::LoginToken::MigrateTo(_) => unreachable!(),
                        }
                    }
                }
            }

            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn logout(&self, account_name: &str) -> Result<()> {
        let (connected, _profile) = self.connect_client(account_name).await?;
        let encrypted_session_path = connected.encrypted_session_path.clone();
        let outcome = async {
            if connected
                .client
                .is_authorized()
                .await
                .map_err(invocation_error)?
            {
                connected
                    .client
                    .sign_out()
                    .await
                    .map_err(invocation_error)?;
            }

            Ok(())
        }
        .await;

        connected.shutdown(&self.secrets).await?;

        if encrypted_session_path.exists() {
            std::fs::remove_file(&encrypted_session_path)?;
        }

        outcome
    }

    async fn resolve_peer(&self, account_name: &str, query: &str) -> Result<ResolvedPeer> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let normalized = query.trim().trim_start_matches('@');

        let outcome = async {
            if let Some(peer) = connected
                .client
                .resolve_username(normalized)
                .await
                .map_err(invocation_error)?
            {
                return Ok(Self::map_peer(&peer));
            }

            let peer = connected
                .client
                .search_peer(normalized, 5)
                .await
                .map_err(invocation_error)?
                .into_iter()
                .next()
                .map(|item| item.into_peer())
                .ok_or_else(|| TelegramCliError::Message(format!("peer {query} was not found")))?;

            Ok(Self::map_peer(&peer))
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn send_text(
        &self,
        account_name: &str,
        peer_id: i64,
        text: &str,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let mut input = InputMessage::new().text(text);
            if let Some(rid) = reply_to {
                input = input.reply_to(Some(rid));
            }
            if let Some(config) = reply_markup {
                if let Some(markup) = config.to_reply_markup() {
                    input = input.reply_markup(markup);
                }
            }
            let message = connected
                .client
                .send_message(peer, input)
                .await
                .map_err(invocation_error)?;

            Ok(SentMessage {
                message_id: i64::from(message.id()),
                account_name: account_name.to_string(),
                peer_id,
                text: Some(text.to_string()),
                file_path: None,
                caption: None,
                timestamp: message.date().to_rfc3339(),
            })
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn send_file(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let uploaded = connected.client.upload_file(path).await.map_err(|error| {
                TelegramCliError::Message(format!("failed to upload {}: {error}", path.display()))
            })?;
            let mut input = match caption {
                Some(caption) => InputMessage::new().text(caption).file(uploaded),
                None => InputMessage::new().file(uploaded),
            };
            if let Some(rid) = reply_to {
                input = input.reply_to(Some(rid));
            }
            if let Some(config) = reply_markup {
                if let Some(markup) = config.to_reply_markup() {
                    input = input.reply_markup(markup);
                }
            }
            let message = connected
                .client
                .send_message(peer, input)
                .await
                .map_err(invocation_error)?;

            Ok(SentMessage {
                message_id: i64::from(message.id()),
                account_name: account_name.to_string(),
                peer_id,
                text: None,
                file_path: Some(path.display().to_string()),
                caption: caption.map(ToOwned::to_owned),
                timestamp: message.date().to_rfc3339(),
            })
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn send_photo(
        &self,
        account_name: &str,
        peer_id: i64,
        path: &Path,
        caption: Option<&str>,
        reply_to: Option<i32>,
        reply_markup: Option<&ReplyMarkupConfig>,
    ) -> Result<SentMessage> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let uploaded = connected.client.upload_file(path).await.map_err(|error| {
                TelegramCliError::Message(format!("failed to upload {}: {error}", path.display()))
            })?;
            let mut input = InputMessage::new();
            if let Some(cap) = caption {
                input = input.text(cap);
            }
            input = input.photo(uploaded);
            if let Some(rid) = reply_to {
                input = input.reply_to(Some(rid));
            }
            if let Some(config) = reply_markup {
                if let Some(markup) = config.to_reply_markup() {
                    input = input.reply_markup(markup);
                }
            }
            let message = connected
                .client
                .send_message(peer, input)
                .await
                .map_err(invocation_error)?;

            Ok(SentMessage {
                message_id: i64::from(message.id()),
                account_name: account_name.to_string(),
                peer_id,
                text: caption.map(ToOwned::to_owned),
                file_path: Some(path.display().to_string()),
                caption: caption.map(ToOwned::to_owned),
                timestamp: message.date().to_rfc3339(),
            })
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn forward_messages(
        &self,
        account_name: &str,
        from_peer_id: i64,
        to_peer_id: i64,
        message_ids: &[i32],
    ) -> Result<Vec<i64>> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let source = Self::peer_ref_for_dialog_id(&connected, from_peer_id).await?;
            let dest = Self::peer_ref_for_dialog_id(&connected, to_peer_id).await?;
            let ids: Vec<i32> = message_ids.to_vec();
            let results = connected
                .client
                .forward_messages(dest, &ids, source)
                .await
                .map_err(invocation_error)?;
            Ok(results
                .into_iter()
                .filter_map(|msg| msg.map(|m| i64::from(m.id())))
                .collect())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn edit_message(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: i32,
        text: &str,
    ) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            connected
                .client
                .edit_message(peer, message_id, InputMessage::new().text(text))
                .await
                .map_err(invocation_error)?;
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn pin_message(&self, account_name: &str, peer_id: i64, message_id: i32) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            connected
                .client
                .pin_message(peer, message_id)
                .await
                .map_err(invocation_error)?;
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn unpin_message(&self, account_name: &str, peer_id: i64, message_id: i32) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            connected
                .client
                .unpin_message(peer, message_id)
                .await
                .map_err(invocation_error)?;
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn download_media(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: i32,
        output_path: &Path,
    ) -> Result<bool> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let messages = connected
                .client
                .get_messages_by_id(peer, &[message_id])
                .await
                .map_err(invocation_error)?;
            if let Some(Some(msg)) = messages.into_iter().next() {
                msg.download_media(output_path)
                    .await
                    .map_err(|e| TelegramCliError::Message(format!("download failed: {e}")))?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn recent_messages(
        &self,
        account_name: &str,
        peer_id: i64,
        limit: usize,
        offset_id: Option<i64>,
        unread_only: bool,
    ) -> Result<Vec<IncomingMessage>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;

            // Build iterator with optional offset
            let mut iter = connected.client.iter_messages(peer).limit(limit);
            if let Some(offset) = offset_id {
                iter = iter.offset_id(offset as i32);
            }

            let mut messages = Vec::new();
            while let Some(message) = iter.next().await.map_err(invocation_error)? {
                // Filter outgoing messages if unread_only is true
                if unread_only && message.outgoing() {
                    continue;
                }
                messages.push(map_incoming_message(&message));
            }
            Ok(messages)
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn wait_for_message(
        &self,
        account_name: &str,
        peer_id: i64,
        filter: &MessageFilter,
        timeout: Duration,
    ) -> Result<IncomingMessage> {
        let (mut connected, _profile) = self.connect_authorized_client(account_name).await?;
        let updates = connected.take_updates()?;
        let mut stream = connected
            .client
            .stream_updates(
                updates,
                UpdatesConfiguration {
                    catch_up: false,
                    ..Default::default()
                },
            )
            .await;

        let deadline = tokio::time::Instant::now() + timeout;
        let outcome = async {
            loop {
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    return Err(TelegramCliError::Message(format!(
                        "timed out waiting for message after {:?}",
                        timeout
                    )));
                }

                let remaining = deadline.saturating_duration_since(now);
                let update = tokio::time::timeout(remaining, stream.next())
                    .await
                    .map_err(|_| {
                        TelegramCliError::Message(format!(
                            "timed out waiting for message after {:?}",
                            timeout
                        ))
                    })?
                    .map_err(invocation_error)?;

                match update {
                    Update::NewMessage(message) if !message.outgoing() => {
                        if message.peer_id().bot_api_dialog_id() != peer_id {
                            continue;
                        }

                        let incoming = map_incoming_message(&message);
                        if filter.matches(&incoming) {
                            return Ok(incoming);
                        }
                    }
                    _ => {}
                }
            }
        }
        .await;

        stream.sync_update_state().await;
        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn delete_message(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: i64,
    ) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let message_id = i32::try_from(message_id).map_err(|_| {
                TelegramCliError::Message(format!("message id {message_id} is out of range"))
            })?;
            connected
                .client
                .delete_messages(peer, &[message_id])
                .await
                .map_err(invocation_error)?;
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn click_button(
        &self,
        account_name: &str,
        peer_id: i64,
        button: &str,
        message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<Option<crate::telegram::IncomingMessage>> {
        let (mut connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let updates_receiver = connected.take_updates()?;
            let mut stream = connected
                .client
                .stream_updates(
                    updates_receiver,
                    UpdatesConfiguration {
                        catch_up: false,
                        ..Default::default()
                    },
                )
                .await;

            let (target_msg_id, callback_data) =
                find_callback_target(&connected.client, peer, button, message_id).await?;

            connected
                .client
                .invoke(&tl::functions::messages::GetBotCallbackAnswer {
                    game: false,
                    peer: peer.into(),
                    msg_id: target_msg_id,
                    data: Some(callback_data),
                    password: None,
                })
                .await
                .map_err(invocation_error)?;

            let deadline = tokio::time::Instant::now() + wait_timeout;
            loop {
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    return Ok(None);
                }

                let remaining = deadline.saturating_duration_since(now);
                let update = tokio::time::timeout(remaining, stream.next())
                    .await
                    .map_err(|_| {
                        TelegramCliError::Message(format!(
                            "timed out waiting for bot response after {:?}",
                            wait_timeout
                        ))
                    })?
                    .map_err(invocation_error)?;

                match update {
                    grammers_client::update::Update::NewMessage(msg) if !msg.outgoing() => {
                        if msg.peer_id().bot_api_dialog_id() == peer_id {
                            return Ok(Some(map_incoming_message(&msg)));
                        }
                    }
                    grammers_client::update::Update::MessageEdited(msg) => {
                        if msg.peer_id().bot_api_dialog_id() == peer_id {
                            return Ok(Some(map_incoming_message(&msg)));
                        }
                    }
                    _ => {}
                }
            }
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn list_contacts(&self, account_name: &str) -> Result<Vec<crate::telegram::Contact>> {
        use crate::telegram::Contact;
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let result = connected
                .client
                .invoke(&tl::functions::contacts::GetContacts { hash: 0 })
                .await
                .map_err(invocation_error)?;

            let contacts = match result {
                tl::enums::contacts::Contacts::Contacts(c) => c,
                tl::enums::contacts::Contacts::NotModified => {
                    return Err(TelegramCliError::Message(
                        "contacts not modified (unexpected response)".into(),
                    ))
                }
            };

            let mut list = Vec::new();
            for user in contacts.users {
                match user {
                    tl::enums::User::User(u) => {
                        let display_name = u
                            .first_name
                            .as_ref()
                            .or(u.last_name.as_ref())
                            .or(u.username.as_ref())
                            .cloned()
                            .unwrap_or_else(|| format!("user_{}", u.id));
                        list.push(Contact {
                            id: u.id,
                            phone: u.phone,
                            display_name,
                            username: u.username,
                        });
                    }
                    tl::enums::User::Empty(u) => {
                        let display_name = format!("user_{}", u.id);
                        list.push(Contact {
                            id: u.id,
                            phone: None,
                            display_name,
                            username: None,
                        });
                    }
                }
            }
            Ok(list)
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn list_chats(&self, account_name: &str) -> Result<Vec<crate::telegram::Chat>> {
        use crate::telegram::{Chat, ChatKind};
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let mut chats = Vec::new();

            // Iterate through all dialogs to find groups and channels
            let mut iter = connected.client.iter_dialogs();
            while let Some(dialog) = iter.next().await.map_err(invocation_error)? {
                let peer = dialog.peer();
                match peer {
                    Peer::Group(_) => {
                        let resolved = Self::map_peer(peer);
                        chats.push(Chat {
                            id: peer.id().bot_api_dialog_id(),
                            kind: ChatKind::Group,
                            display_name: resolved.display_name,
                            username: peer.username().map(|s| s.to_owned()),
                        });
                    }
                    Peer::Channel(_) => {
                        let resolved = Self::map_peer(peer);
                        chats.push(Chat {
                            id: peer.id().bot_api_dialog_id(),
                            kind: ChatKind::Channel,
                            display_name: resolved.display_name,
                            username: peer.username().map(|s| s.to_owned()),
                        });
                    }
                    _ => {}
                }
            }

            Ok(chats)
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn set_bot_commands(
        &self,
        account_name: &str,
        commands: &[(String, String)],
    ) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let scope: tl::enums::BotCommandScope = tl::types::BotCommandScopeDefault {}.into();
            let tl_commands: Vec<tl::enums::BotCommand> = commands
                .iter()
                .map(|(name, desc)| {
                    tl::enums::BotCommand::Command(tl::types::BotCommand {
                        command: name.clone(),
                        description: desc.clone(),
                    })
                })
                .collect();
            connected
                .client
                .invoke(&tl::functions::bots::SetBotCommands {
                    scope,
                    lang_code: String::new(),
                    commands: tl_commands,
                })
                .await
                .map_err(invocation_error)?;
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn set_bot_info(
        &self,
        account_name: &str,
        description: Option<&str>,
        about: Option<&str>,
    ) -> Result<()> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            if let Some(desc) = description {
                connected
                    .client
                    .invoke(&tl::functions::bots::SetBotInfo {
                        bot: None,
                        lang_code: String::new(),
                        name: None,
                        description: Some(desc.to_string()),
                        about: None,
                    })
                    .await
                    .map_err(invocation_error)?;
            }
            if let Some(about_text) = about {
                connected
                    .client
                    .invoke(&tl::functions::bots::SetBotInfo {
                        bot: None,
                        lang_code: String::new(),
                        name: None,
                        description: None,
                        about: Some(about_text.to_string()),
                    })
                    .await
                    .map_err(invocation_error)?;
            }
            Ok(())
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn list_actions(
        &self,
        account_name: &str,
        peer_id: i64,
        message_id: Option<i64>,
        search_limit: usize,
    ) -> Result<Vec<InteractiveAction>> {
        let (connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let mut actions =
                collect_message_actions(&connected.client, peer, search_limit, message_id).await?;
            actions.extend(collect_bot_profile_actions(&connected.client, peer).await?);
            Ok(actions)
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }

    async fn trigger_action(
        &self,
        account_name: &str,
        peer_id: i64,
        action: &str,
        message_id: Option<i64>,
        wait_timeout: Duration,
    ) -> Result<ActionInvocationResult> {
        let (mut connected, _profile) = self.connect_authorized_client(account_name).await?;
        let outcome = async {
            let peer = Self::peer_ref_for_dialog_id(&connected, peer_id).await?;
            let mut actions =
                collect_message_actions(&connected.client, peer, 50, message_id).await?;
            actions.extend(collect_bot_profile_actions(&connected.client, peer).await?);
            let matched = resolve_action_match(&actions, action)?;

            if !matched.supported {
                return Err(TelegramCliError::Message(
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
                    let message_id = matched.message_id.ok_or_else(|| {
                        TelegramCliError::Message(
                            "inline callback action is missing its message id".into(),
                        )
                    })?;
                    let payload = matched
                        .callback_data_base64
                        .as_ref()
                        .ok_or_else(|| {
                            TelegramCliError::Message(
                                "inline callback action is missing its callback payload".into(),
                            )
                        })
                        .and_then(|encoded| {
                            BASE64URL.decode(encoded).map_err(|error| {
                                TelegramCliError::Message(format!(
                                    "failed to decode callback payload `{encoded}`: {error}"
                                ))
                            })
                        })?;
                    let updates_receiver = connected.take_updates()?;
                    let mut stream = connected
                        .client
                        .stream_updates(
                            updates_receiver,
                            UpdatesConfiguration {
                                catch_up: false,
                                ..Default::default()
                            },
                        )
                        .await;

                    connected
                        .client
                        .invoke(&tl::functions::messages::GetBotCallbackAnswer {
                            game: false,
                            peer: peer.into(),
                            msg_id: message_id as i32,
                            data: Some(payload),
                            password: None,
                        })
                        .await
                        .map_err(invocation_error)?;

                    let response =
                        wait_for_peer_response(&mut stream, peer_id, wait_timeout).await?;
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
                    let updates_receiver = connected.take_updates()?;
                    let mut stream = connected
                        .client
                        .stream_updates(
                            updates_receiver,
                            UpdatesConfiguration {
                                catch_up: false,
                                ..Default::default()
                            },
                        )
                        .await;

                    connected
                        .client
                        .send_message(peer, sent_text.as_str())
                        .await
                        .map_err(invocation_error)?;

                    let response =
                        wait_for_peer_response(&mut stream, peer_id, wait_timeout).await?;
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
                _ => Err(TelegramCliError::Message(format!(
                    "action `{}` is not currently triggerable by telegram-agent-cli",
                    matched.label
                ))),
            }
        }
        .await;

        merge_shutdown(outcome, connected.shutdown(&self.secrets).await)
    }
}

fn resolve_api_credentials(
    profile: &AccountProfile,
    env_api_id: Option<&str>,
    env_api_hash: Option<&str>,
) -> Result<ApiCredentials> {
    let api_id = match profile.api_id {
        Some(api_id) => api_id,
        None => env_api_id
            .ok_or_else(|| {
                TelegramCliError::Message(format!(
                    "account {} is missing API ID; set --api-id when creating it or export {}",
                    profile.name, ENV_API_ID
                ))
            })?
            .parse::<i32>()
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "{} must be a valid integer API ID: {error}",
                    ENV_API_ID
                ))
            })?,
    };

    let api_hash = profile
        .api_hash
        .clone()
        .or_else(|| env_api_hash.map(ToOwned::to_owned))
        .ok_or_else(|| {
            TelegramCliError::Message(format!(
                "account {} is missing API hash; set --api-hash when creating it or export {}",
                profile.name, ENV_API_HASH
            ))
        })?;

    Ok(ApiCredentials { api_id, api_hash })
}

async fn find_callback_target(
    client: &Client,
    peer: PeerRef,
    button_query: &str,
    message_id: Option<i64>,
) -> Result<(i32, Vec<u8>)> {
    let search_limit = if message_id.is_some() { 100 } else { 50 };
    let mut iter = client.iter_messages(peer).limit(search_limit);

    while let Some(message) = iter.next().await.map_err(invocation_error)? {
        if let Some(expected_id) = message_id {
            if i64::from(message.id()) != expected_id {
                continue;
            }
        }

        if let Some(callback_data) =
            find_callback_data_in_markup(message.reply_markup(), button_query)
        {
            return Ok((message.id(), callback_data));
        }
    }

    match message_id {
        Some(message_id) => Err(TelegramCliError::Message(format!(
            "message {message_id} does not contain a clickable inline button matching `{button_query}`"
        ))),
        None => Err(TelegramCliError::Message(format!(
            "no recent message found with an inline button matching `{button_query}`; specify --message-id if needed"
        ))),
    }
}

fn find_callback_data_in_markup(
    reply_markup: Option<tl::enums::ReplyMarkup>,
    button_query: &str,
) -> Option<Vec<u8>> {
    let markup = tl::types::ReplyInlineMarkup::try_from(reply_markup?).ok()?;
    for row in markup.rows {
        let row = tl::types::KeyboardButtonRow::from(row);
        for button in row.buttons {
            if let Ok(callback) = tl::types::KeyboardButtonCallback::try_from(button) {
                if callback_button_matches(&callback, button_query) {
                    return Some(callback.data);
                }
            }
        }
    }
    None
}

fn callback_button_matches(button: &tl::types::KeyboardButtonCallback, button_query: &str) -> bool {
    if button.text == button_query || button.data == button_query.as_bytes() {
        return true;
    }

    BASE64URL
        .decode(button_query)
        .map(|decoded| decoded == button.data)
        .unwrap_or(false)
}

async fn collect_message_actions(
    client: &Client,
    peer: PeerRef,
    search_limit: usize,
    message_id: Option<i64>,
) -> Result<Vec<InteractiveAction>> {
    if search_limit == 0 && message_id.is_none() {
        return Ok(Vec::new());
    }

    let limit = if message_id.is_some() {
        search_limit.max(100).max(1)
    } else {
        search_limit.max(1)
    };
    let mut iter = client.iter_messages(peer).limit(limit);
    let mut actions = Vec::new();

    while let Some(message) = iter.next().await.map_err(invocation_error)? {
        let current_message_id = i64::from(message.id());
        if let Some(expected_message_id) = message_id {
            if current_message_id != expected_message_id {
                continue;
            }
        }

        actions.extend(actions_from_reply_markup(
            current_message_id,
            message.reply_markup(),
        ));

        if message_id.is_some() {
            break;
        }
    }

    Ok(actions)
}

fn actions_from_reply_markup(
    message_id: i64,
    reply_markup: Option<tl::enums::ReplyMarkup>,
) -> Vec<InteractiveAction> {
    match reply_markup {
        Some(tl::enums::ReplyMarkup::ReplyInlineMarkup(markup)) => markup
            .rows
            .into_iter()
            .flat_map(|row| {
                let row = tl::types::KeyboardButtonRow::from(row);
                row.buttons
                    .into_iter()
                    .filter_map(move |button| inline_action_from_button(message_id, button))
            })
            .collect(),
        Some(tl::enums::ReplyMarkup::ReplyKeyboardMarkup(markup)) => markup
            .rows
            .into_iter()
            .flat_map(|row| {
                let row = tl::types::KeyboardButtonRow::from(row);
                row.buttons
                    .into_iter()
                    .filter_map(move |button| reply_action_from_button(message_id, button))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn inline_action_from_button(
    message_id: i64,
    button: tl::enums::KeyboardButton,
) -> Option<InteractiveAction> {
    match button {
        tl::enums::KeyboardButton::Callback(button) => Some(inline_callback_action(
            message_id,
            &button.text,
            &button.data,
        )),
        tl::enums::KeyboardButton::Url(button) => {
            Some(inline_url_action(message_id, &button.text, &button.url))
        }
        tl::enums::KeyboardButton::UrlAuth(button) => {
            Some(inline_url_action(message_id, &button.text, &button.url))
        }
        tl::enums::KeyboardButton::WebView(button) => {
            Some(inline_url_action(message_id, &button.text, &button.url))
        }
        tl::enums::KeyboardButton::SimpleWebView(button) => {
            Some(inline_url_action(message_id, &button.text, &button.url))
        }
        _ => None,
    }
}

fn reply_action_from_button(
    message_id: i64,
    button: tl::enums::KeyboardButton,
) -> Option<InteractiveAction> {
    match button {
        tl::enums::KeyboardButton::Button(button) => {
            Some(reply_keyboard_text_action(message_id, &button.text))
        }
        tl::enums::KeyboardButton::RequestPhone(button) => Some(unsupported_reply_action(
            message_id,
            InteractiveActionKind::ReplyKeyboardRequestPhone,
            &button.text,
            "reply keyboard phone requests require contact-sharing support, which telegram-agent-cli does not automate yet",
        )),
        tl::enums::KeyboardButton::RequestGeoLocation(button) => Some(unsupported_reply_action(
            message_id,
            InteractiveActionKind::ReplyKeyboardRequestGeo,
            &button.text,
            "reply keyboard geo requests require location input, which telegram-agent-cli does not automate yet",
        )),
        tl::enums::KeyboardButton::RequestPoll(button) => Some(unsupported_reply_action(
            message_id,
            InteractiveActionKind::ReplyKeyboardRequestPoll,
            &button.text,
            "reply keyboard poll requests require poll creation, which telegram-agent-cli does not automate yet",
        )),
        tl::enums::KeyboardButton::RequestPeer(button) => Some(unsupported_reply_action(
            message_id,
            InteractiveActionKind::ReplyKeyboardRequestPeer,
            &button.text,
            "reply keyboard peer requests require peer-selection input, which telegram-agent-cli does not automate yet",
        )),
        _ => None,
    }
}

async fn collect_bot_profile_actions(
    client: &Client,
    peer: PeerRef,
) -> Result<Vec<InteractiveAction>> {
    match peer.id.kind() {
        SessionPeerKind::User | SessionPeerKind::UserSelf => {
            let full: tl::types::users::UserFull = client
                .invoke(&tl::functions::users::GetFullUser { id: peer.into() })
                .await
                .map_err(invocation_error)?
                .into();
            let tl::enums::UserFull::Full(full_user) = full.full_user;
            let bot_info = match full_user.bot_info {
                Some(tl::enums::BotInfo::Info(bot_info)) => bot_info,
                None => return Ok(Vec::new()),
            };
            Ok(actions_from_bot_info(&bot_info, &full.users, false))
        }
        SessionPeerKind::Chat => {
            let full: tl::types::messages::ChatFull = client
                .invoke(&tl::functions::messages::GetFullChat {
                    chat_id: peer.id.bare_id(),
                })
                .await
                .map_err(invocation_error)?
                .into();
            Ok(actions_from_chat_full(&full.full_chat, &full.users, true))
        }
        SessionPeerKind::Channel => {
            let full: tl::types::messages::ChatFull = client
                .invoke(&tl::functions::channels::GetFullChannel {
                    channel: peer.into(),
                })
                .await
                .map_err(invocation_error)?
                .into();
            Ok(actions_from_chat_full(&full.full_chat, &full.users, true))
        }
    }
}

fn actions_from_chat_full(
    full_chat: &tl::enums::ChatFull,
    users: &[tl::enums::User],
    qualify_commands: bool,
) -> Vec<InteractiveAction> {
    match full_chat {
        tl::enums::ChatFull::Full(chat) => chat
            .bot_info
            .as_ref()
            .map(|bot_infos| {
                bot_infos
                    .iter()
                    .flat_map(|info| match info {
                        tl::enums::BotInfo::Info(bot_info) => {
                            actions_from_bot_info(bot_info, users, qualify_commands)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        tl::enums::ChatFull::ChannelFull(channel) => channel
            .bot_info
            .iter()
            .flat_map(|info| match info {
                tl::enums::BotInfo::Info(bot_info) => {
                    actions_from_bot_info(bot_info, users, qualify_commands)
                }
            })
            .collect(),
    }
}

fn actions_from_bot_info(
    bot_info: &tl::types::BotInfo,
    users: &[tl::enums::User],
    qualify_commands: bool,
) -> Vec<InteractiveAction> {
    let bot_user_id = bot_info.user_id;
    let bot_username = bot_user_id.and_then(|user_id| lookup_bot_username(users, user_id));
    let bot_identity = bot_username
        .clone()
        .or_else(|| bot_user_id.map(|id| format!("bot_{id}")))
        .unwrap_or_else(|| "bot".into());
    let mut actions = Vec::new();

    if let Some(commands) = &bot_info.commands {
        for command in commands {
            let tl::enums::BotCommand::Command(command) = command;
            let display_command = format_command_for_peer(
                &command.command,
                bot_username.as_deref(),
                qualify_commands,
            );
            let supported = !qualify_commands || bot_username.is_some();
            let unsupported_reason = (!supported).then(|| {
                "bot commands in group or channel chats require a bot username, but Telegram did not provide one".to_string()
            });
            actions.push(bot_command_action(
                &bot_identity,
                &display_command,
                &command.description,
                supported,
                unsupported_reason,
            ));
        }
    }

    if let Some(tl::enums::BotMenuButton::Button(button)) = &bot_info.menu_button {
        actions.push(bot_menu_url_action(
            &bot_identity,
            &button.text,
            &button.url,
        ));
    }

    actions
}

fn lookup_bot_username(users: &[tl::enums::User], user_id: i64) -> Option<String> {
    users.iter().find_map(|user| match user {
        tl::enums::User::User(user) if user.id == user_id => user.username.clone(),
        _ => None,
    })
}

fn format_command_for_peer(
    command: &str,
    bot_username: Option<&str>,
    qualify_commands: bool,
) -> String {
    let normalized = normalize_command(command);
    if qualify_commands {
        bot_username
            .map(|username| format!("{normalized}@{username}"))
            .unwrap_or(normalized)
    } else {
        normalized
    }
}

fn inline_callback_action(message_id: i64, label: &str, payload: &[u8]) -> InteractiveAction {
    let callback_data_base64 = BASE64URL.encode(payload);
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

fn unsupported_reply_action(
    message_id: i64,
    action_kind: InteractiveActionKind,
    label: &str,
    reason: &str,
) -> InteractiveAction {
    InteractiveAction {
        action_id: format!(
            "reply-unsupported:{message_id}:{}:{}",
            action_kind_label(action_kind),
            BASE64URL.encode(label.as_bytes())
        ),
        action_kind,
        source: InteractiveActionSource::MessageMarkup,
        label: label.to_string(),
        description: None,
        message_id: Some(message_id),
        bot_username: None,
        callback_data_base64: None,
        command: None,
        trigger_text: None,
        url: None,
        supported: false,
        unsupported_reason: Some(reason.to_string()),
    }
}

fn bot_command_action(
    bot_identity: &str,
    command: &str,
    description: &str,
    supported: bool,
    unsupported_reason: Option<String>,
) -> InteractiveAction {
    let normalized = normalize_command(command);
    InteractiveAction {
        action_id: format!(
            "bot-command:{bot_identity}:{}",
            normalized.trim_start_matches('/')
        ),
        action_kind: InteractiveActionKind::BotCommand,
        source: InteractiveActionSource::BotProfile,
        label: normalized.clone(),
        description: Some(description.to_string()),
        message_id: None,
        bot_username: Some(bot_identity.to_string()),
        callback_data_base64: None,
        command: Some(normalized.clone()),
        trigger_text: Some(normalized),
        url: None,
        supported,
        unsupported_reason,
    }
}

fn bot_menu_url_action(bot_identity: &str, label: &str, url: &str) -> InteractiveAction {
    InteractiveAction {
        action_id: format!(
            "bot-menu-url:{bot_identity}:{}",
            BASE64URL.encode(url.as_bytes())
        ),
        action_kind: InteractiveActionKind::BotMenuUrl,
        source: InteractiveActionSource::BotProfile,
        label: label.to_string(),
        description: None,
        message_id: None,
        bot_username: Some(bot_identity.to_string()),
        callback_data_base64: None,
        command: None,
        trigger_text: None,
        url: Some(url.to_string()),
        supported: true,
        unsupported_reason: None,
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
        0 => Err(TelegramCliError::Message(format!(
            "action `{query}` was not found; run `telegram-agent-cli message list-actions --chat <peer>` to inspect available actions"
        ))),
        1 => Ok(matches[0]),
        _ => Err(TelegramCliError::Message(format!(
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

fn normalize_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn action_kind_label(kind: InteractiveActionKind) -> &'static str {
    match kind {
        InteractiveActionKind::InlineCallback => "inline_callback",
        InteractiveActionKind::InlineUrl => "inline_url",
        InteractiveActionKind::ReplyKeyboardText => "reply_keyboard_text",
        InteractiveActionKind::ReplyKeyboardRequestPhone => "reply_keyboard_request_phone",
        InteractiveActionKind::ReplyKeyboardRequestGeo => "reply_keyboard_request_geo",
        InteractiveActionKind::ReplyKeyboardRequestPoll => "reply_keyboard_request_poll",
        InteractiveActionKind::ReplyKeyboardRequestPeer => "reply_keyboard_request_peer",
        InteractiveActionKind::BotCommand => "bot_command",
        InteractiveActionKind::BotMenuUrl => "bot_menu_url",
    }
}

async fn wait_for_peer_response(
    stream: &mut grammers_client::client::UpdateStream,
    peer_id: i64,
    wait_timeout: Duration,
) -> Result<Option<IncomingMessage>> {
    let deadline = tokio::time::Instant::now() + wait_timeout;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Ok(None);
        }

        let remaining = deadline.saturating_duration_since(now);
        let update = match tokio::time::timeout(remaining, stream.next()).await {
            Ok(result) => result.map_err(invocation_error)?,
            Err(_) => return Ok(None),
        };

        match update {
            Update::NewMessage(message) if !message.outgoing() => {
                if message.peer_id().bot_api_dialog_id() == peer_id {
                    return Ok(Some(map_incoming_message(&message)));
                }
            }
            Update::MessageEdited(message) => {
                if message.peer_id().bot_api_dialog_id() == peer_id {
                    return Ok(Some(map_incoming_message(&message)));
                }
            }
            _ => {}
        }
    }
}

fn peer_id_from_dialog_id(dialog_id: i64) -> Result<PeerId> {
    if let Some(peer_id) = PeerId::user(dialog_id) {
        return Ok(peer_id);
    }

    if dialog_id < 0 && dialog_id > -1_000_000_000_000 {
        return PeerId::chat(-dialog_id).ok_or_else(|| {
            TelegramCliError::Message(format!("unsupported chat peer id {dialog_id}"))
        });
    }

    if dialog_id <= -1_000_000_000_001 {
        let bare_id = -dialog_id - 1_000_000_000_000;
        return PeerId::channel(bare_id).ok_or_else(|| {
            TelegramCliError::Message(format!("unsupported channel peer id {dialog_id}"))
        });
    }

    Err(TelegramCliError::Message(format!(
        "unsupported peer id {dialog_id}"
    )))
}

fn map_incoming_message(message: &grammers_client::message::Message) -> IncomingMessage {
    let sender = message.sender().map(|peer| {
        peer.username()
            .or_else(|| peer.name())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| peer.id().bot_api_dialog_id().to_string())
    });
    let text = (!message.text().is_empty()).then(|| message.text().to_string());
    let kind = if message.media().is_some() {
        MessageKind::File
    } else {
        MessageKind::Text
    };

    IncomingMessage {
        message_id: i64::from(message.id()),
        peer_id: message.peer_id().bot_api_dialog_id(),
        sender,
        text,
        kind,
        reply_to_message_id: message.reply_to_message_id().map(i64::from),
        timestamp: message.date().to_rfc3339(),
    }
}

fn sanitize_account_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn persist_encrypted_session_snapshot(
    secrets: &SecretStore,
    session: &PersistedSessionStore,
    encrypted_path: &Path,
) -> Result<()> {
    let payload = serde_json::to_vec(&session.snapshot()).map_err(|error| {
        TelegramCliError::Message(format!(
            "failed to serialize Telegram session state: {error}"
        ))
    })?;
    let encrypted = secrets.encrypt_bytes(&payload)?;
    let mut blob = Vec::with_capacity(encrypted.nonce.len() + encrypted.ciphertext.len());
    blob.extend_from_slice(&encrypted.nonce);
    blob.extend_from_slice(&encrypted.ciphertext);
    std::fs::write(encrypted_path, blob)?;
    Ok(())
}

fn load_encrypted_session_snapshot(
    secrets: &SecretStore,
    encrypted_path: &Path,
) -> Result<PersistedSessionStore> {
    if !encrypted_path.exists() {
        return Ok(PersistedSessionStore::from_state(
            PersistedSessionState::default(),
        ));
    }

    let blob = std::fs::read(encrypted_path)?;
    if blob.len() < 12 {
        return Err(TelegramCliError::Message(format!(
            "encrypted session {} is corrupted",
            encrypted_path.display()
        )));
    }

    let encrypted = crate::storage::EncryptedValue {
        nonce: blob[..12].to_vec(),
        ciphertext: blob[12..].to_vec(),
    };
    let payload = secrets.decrypt_bytes(&encrypted)?;

    if let Ok(state) = serde_json::from_slice::<PersistedSessionState>(&payload) {
        return Ok(PersistedSessionStore::from_state(state));
    }

    Ok(PersistedSessionStore::from_state(
        migrate_legacy_sqlite_session(&payload)?,
    ))
}

fn migrate_legacy_sqlite_session(payload: &[u8]) -> Result<PersistedSessionState> {
    let legacy_path = std::env::temp_dir().join(format!(
        "telegram-agent-cli-legacy-session-{}-{}.sqlite3",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::write(&legacy_path, payload)?;

    let result = (|| -> Result<PersistedSessionState> {
        let connection = Connection::open_with_flags(
            &legacy_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            TelegramCliError::Message(format!(
                "failed to open legacy Telegram session snapshot {}: {error}",
                legacy_path.display()
            ))
        })?;

        let mut state = PersistedSessionState::default();
        state.home_dc = connection
            .query_row("SELECT dc_id FROM dc_home LIMIT 1", [], |row| row.get(0))
            .unwrap_or(state.home_dc);

        let mut dc_stmt = connection
            .prepare("SELECT dc_id, ipv4, ipv6, auth_key FROM dc_option")
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to read legacy Telegram datacenter options: {error}"
                ))
            })?;
        let dc_rows = dc_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                ))
            })
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to scan legacy Telegram datacenter options: {error}"
                ))
            })?;

        for row in dc_rows {
            let (id, ipv4, ipv6, auth_key) = row.map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to decode legacy Telegram datacenter option: {error}"
                ))
            })?;
            state.dc_options.insert(
                id,
                DcOption {
                    id,
                    ipv4: ipv4.parse().map_err(|error| {
                        TelegramCliError::Message(format!(
                            "invalid legacy Telegram ipv4 address {ipv4}: {error}"
                        ))
                    })?,
                    ipv6: ipv6.parse().map_err(|error| {
                        TelegramCliError::Message(format!(
                            "invalid legacy Telegram ipv6 address {ipv6}: {error}"
                        ))
                    })?,
                    auth_key: auth_key
                        .map(|bytes| {
                            bytes.try_into().map_err(|_| {
                                TelegramCliError::Message(
                                    "legacy Telegram auth key had an unexpected length".into(),
                                )
                            })
                        })
                        .transpose()?,
                },
            );
        }

        let mut peer_stmt = connection
            .prepare("SELECT peer_id, hash, subtype FROM peer_info")
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to read legacy Telegram peer cache: {error}"
                ))
            })?;
        let peer_rows = peer_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to scan legacy Telegram peer cache: {error}"
                ))
            })?;

        for row in peer_rows {
            let (dialog_id, hash, subtype) = row.map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to decode legacy Telegram peer row: {error}"
                ))
            })?;
            let peer_id = peer_id_from_dialog_id(dialog_id)?;
            let subtype = subtype.map(|value| value as u8);
            let peer_info = match peer_id.kind() {
                SessionPeerKind::User | SessionPeerKind::UserSelf => PeerInfo::User {
                    id: peer_id.bare_id(),
                    auth: hash.map(PeerAuth::from_hash),
                    bot: subtype.map(|bits| bits & 2 != 0),
                    is_self: subtype.map(|bits| bits & 1 != 0),
                },
                SessionPeerKind::Chat => PeerInfo::Chat {
                    id: peer_id.bare_id(),
                },
                SessionPeerKind::Channel => PeerInfo::Channel {
                    id: peer_id.bare_id(),
                    auth: hash.map(PeerAuth::from_hash),
                    kind: subtype.and_then(|bits| {
                        if bits & 12 == 12 {
                            Some(SessionChannelKind::Gigagroup)
                        } else if bits & 8 != 0 {
                            Some(SessionChannelKind::Broadcast)
                        } else if bits & 4 != 0 {
                            Some(SessionChannelKind::Megagroup)
                        } else {
                            None
                        }
                    }),
                },
            };
            state.peer_infos.insert(peer_info.id(), peer_info);
        }

        state.updates_state = connection
            .query_row(
                "SELECT pts, qts, date, seq FROM update_state LIMIT 1",
                [],
                |row| {
                    Ok(UpdatesState {
                        pts: row.get(0)?,
                        qts: row.get(1)?,
                        date: row.get(2)?,
                        seq: row.get(3)?,
                        channels: Vec::new(),
                    })
                },
            )
            .unwrap_or_default();

        let mut channel_stmt = connection
            .prepare("SELECT peer_id, pts FROM channel_state")
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to read legacy Telegram channel state: {error}"
                ))
            })?;
        let channel_rows = channel_stmt
            .query_map([], |row| {
                Ok(ChannelState {
                    id: row.get(0)?,
                    pts: row.get(1)?,
                })
            })
            .map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to scan legacy Telegram channel state: {error}"
                ))
            })?;
        state.updates_state.channels.clear();
        for row in channel_rows {
            state.updates_state.channels.push(row.map_err(|error| {
                TelegramCliError::Message(format!(
                    "failed to decode legacy Telegram channel state: {error}"
                ))
            })?);
        }

        Ok(state)
    })();

    let _ = std::fs::remove_file(&legacy_path);
    result
}

fn merge_shutdown<T>(outcome: Result<T>, shutdown: Result<()>) -> Result<T> {
    match outcome {
        Ok(value) => {
            shutdown?;
            Ok(value)
        }
        Err(error) => {
            let _ = shutdown;
            Err(error)
        }
    }
}

#[cfg(test)]
fn encode_session_blob(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    BASE64.encode(bytes)
}

#[cfg(test)]
fn decode_session_blob(blob: &str) -> Result<Vec<u8>> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    BASE64
        .decode(blob)
        .map_err(|error| TelegramCliError::Message(format!("invalid base64 session blob: {error}")))
}

fn prompt_line(prompt: &str) -> std::io::Result<String> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(prompt.as_bytes())?;
    stdout.flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn render_qr_to_terminal(url: &str) -> Result<()> {
    use qrcode::render::unicode;
    use qrcode::QrCode;
    let code = QrCode::new(url.as_bytes())
        .map_err(|e| TelegramCliError::Message(format!("failed to generate QR code: {e}")))?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    println!("{image}");
    Ok(())
}

fn compute_qr_expiry(expires_unix: i32) -> tokio::time::Instant {
    let now_unix = chrono::Utc::now().timestamp();
    let remaining = (expires_unix as i64 - now_unix).max(0) as u64;
    tokio::time::Instant::now() + Duration::from_secs(remaining)
}

fn expiry_secs(expires_unix: i32) -> i64 {
    (expires_unix as i64 - chrono::Utc::now().timestamp()).max(0)
}

fn invocation_error(error: grammers_client::InvocationError) -> TelegramCliError {
    TelegramCliError::Message(format!("Telegram API request failed: {error}"))
}

fn sign_in_error(error: SignInError) -> TelegramCliError {
    TelegramCliError::Message(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        actions_from_bot_info, actions_from_reply_markup, callback_button_matches,
        decode_session_blob, encode_session_blob, find_callback_data_in_markup,
        load_encrypted_session_snapshot, persist_encrypted_session_snapshot,
        resolve_api_credentials, tl, PersistedSessionState, PersistedSessionStore,
    };
    use crate::storage::{AccountKind, AccountProfile, LoginState, SecretStore};
    use crate::telegram::InteractiveActionKind;

    #[test]
    fn session_blob_roundtrips_through_base64_storage() {
        let encoded = encode_session_blob(&[1, 2, 3, 4]);
        let decoded = decode_session_blob(&encoded).unwrap();
        assert_eq!(decoded, vec![1, 2, 3, 4]);
    }

    #[test]
    fn encrypted_session_snapshot_roundtrips_via_snapshot_file() {
        let temp = tempfile::tempdir().unwrap();
        let encrypted_path = temp.path().join("session.sqlite3.enc");
        let store = SecretStore::from_key_material("test-key");
        let session = PersistedSessionStore::from_state(PersistedSessionState::default());

        persist_encrypted_session_snapshot(&store, &session, &encrypted_path).unwrap();
        assert!(encrypted_path.exists());
        assert_ne!(
            std::fs::read(&encrypted_path).unwrap(),
            serde_json::to_vec(&PersistedSessionState::default()).unwrap()
        );

        let restored = load_encrypted_session_snapshot(&store, &encrypted_path).unwrap();
        assert_eq!(
            restored.snapshot().home_dc,
            PersistedSessionState::default().home_dc
        );
    }

    #[test]
    fn encrypted_session_snapshot_preserves_peer_cache() {
        let temp = tempfile::tempdir().unwrap();
        let encrypted_path = temp.path().join("session.sqlite3.enc");
        let store = SecretStore::from_key_material("test-key");
        let mut state = PersistedSessionState::default();
        let peer = grammers_client::session::types::PeerInfo::Channel {
            id: 42,
            auth: Some(grammers_client::session::types::PeerAuth::from_hash(99)),
            kind: Some(grammers_client::session::types::ChannelKind::Megagroup),
        };
        state.peer_infos.insert(peer.id(), peer.clone());
        let session = PersistedSessionStore::from_state(state);

        persist_encrypted_session_snapshot(&store, &session, &encrypted_path).unwrap();
        let restored = load_encrypted_session_snapshot(&store, &encrypted_path).unwrap();

        assert_eq!(
            restored
                .snapshot()
                .peer_infos
                .get(&peer.id())
                .cloned()
                .expect("peer should roundtrip"),
            peer
        );
    }

    #[test]
    fn load_encrypted_session_snapshot_migrates_legacy_sqlite_payload() {
        let temp = tempfile::tempdir().unwrap();
        let legacy_path = temp.path().join("legacy.sqlite3");
        let encrypted_path = temp.path().join("session.sqlite3.enc");
        let store = SecretStore::from_key_material("test-key");
        let connection = rusqlite::Connection::open(&legacy_path).unwrap();

        connection
            .execute_batch(
                "CREATE TABLE dc_home (
                    dc_id INTEGER NOT NULL,
                    PRIMARY KEY(dc_id)
                );
                CREATE TABLE dc_option (
                    dc_id INTEGER NOT NULL,
                    ipv4 TEXT NOT NULL,
                    ipv6 TEXT NOT NULL,
                    auth_key BLOB,
                    PRIMARY KEY (dc_id)
                );
                CREATE TABLE peer_info (
                    peer_id INTEGER NOT NULL,
                    hash INTEGER,
                    subtype INTEGER,
                    PRIMARY KEY (peer_id)
                );
                CREATE TABLE update_state (
                    pts INTEGER NOT NULL,
                    qts INTEGER NOT NULL,
                    date INTEGER NOT NULL,
                    seq INTEGER NOT NULL
                );
                CREATE TABLE channel_state (
                    peer_id INTEGER NOT NULL,
                    pts INTEGER NOT NULL,
                    PRIMARY KEY (peer_id)
                );",
            )
            .unwrap();
        connection
            .execute("INSERT INTO dc_home VALUES (?1)", [7_i32])
            .unwrap();
        connection
            .execute(
                "INSERT INTO dc_option VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![7_i32, "127.0.0.1:443", "[::1]:443", vec![1_u8; 256]],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO peer_info VALUES (?1, ?2, ?3)",
                rusqlite::params![-1_000_000_000_042_i64, 99_i64, 4_i64],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO update_state VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![11_i32, 12_i32, 13_i32, 14_i32],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO channel_state VALUES (?1, ?2)",
                rusqlite::params![42_i64, 33_i32],
            )
            .unwrap();
        drop(connection);

        let payload = std::fs::read(&legacy_path).unwrap();
        let encrypted = store.encrypt_bytes(&payload).unwrap();
        let mut blob = encrypted.nonce;
        blob.extend_from_slice(&encrypted.ciphertext);
        std::fs::write(&encrypted_path, blob).unwrap();

        let restored = load_encrypted_session_snapshot(&store, &encrypted_path).unwrap();
        let snapshot = restored.snapshot();
        let migrated_peer = snapshot
            .peer_infos
            .get(&grammers_client::session::types::PeerId::channel(42).unwrap())
            .cloned()
            .expect("legacy peer should migrate");

        assert_eq!(snapshot.home_dc, 7);
        assert_eq!(
            snapshot.dc_options.get(&7).unwrap().ipv4.to_string(),
            "127.0.0.1:443"
        );
        assert_eq!(
            snapshot.dc_options.get(&7).unwrap().ipv6.to_string(),
            "[::1]:443"
        );
        assert_eq!(
            snapshot.dc_options.get(&7).unwrap().auth_key,
            Some([1_u8; 256])
        );
        assert_eq!(
            migrated_peer,
            grammers_client::session::types::PeerInfo::Channel {
                id: 42,
                auth: Some(grammers_client::session::types::PeerAuth::from_hash(99)),
                kind: Some(grammers_client::session::types::ChannelKind::Megagroup),
            }
        );
        assert_eq!(snapshot.updates_state.pts, 11);
        assert_eq!(snapshot.updates_state.qts, 12);
        assert_eq!(snapshot.updates_state.date, 13);
        assert_eq!(snapshot.updates_state.seq, 14);
        assert_eq!(
            snapshot.updates_state.channels,
            vec![super::ChannelState { id: 42, pts: 33 }]
        );
    }

    #[test]
    fn resolve_api_credentials_prefers_profile_values() {
        let profile = AccountProfile {
            id: 1,
            name: "alice".into(),
            kind: AccountKind::User,
            login_state: LoginState::Pending,
            is_default: true,
            api_id: Some(1001),
            api_hash: Some("stored-hash".into()),
            phone: Some("+10000000000".into()),
            bot_token: None,
            last_login_at: None,
        };

        let credentials =
            resolve_api_credentials(&profile, Some("2002"), Some("env-hash")).unwrap();
        assert_eq!(credentials.api_id, 1001);
        assert_eq!(credentials.api_hash, "stored-hash");
    }

    #[test]
    fn resolve_api_credentials_uses_env_fallback_when_profile_missing() {
        let profile = AccountProfile {
            id: 1,
            name: "bot".into(),
            kind: AccountKind::Bot,
            login_state: LoginState::Pending,
            is_default: true,
            api_id: None,
            api_hash: None,
            phone: None,
            bot_token: Some("12345:token".into()),
            last_login_at: None,
        };

        let credentials =
            resolve_api_credentials(&profile, Some("2002"), Some("env-hash")).unwrap();
        assert_eq!(credentials.api_id, 2002);
        assert_eq!(credentials.api_hash, "env-hash");
    }

    #[test]
    fn callback_button_matches_text_raw_and_base64_queries() {
        let callback = tl::types::KeyboardButtonCallback {
            requires_password: false,
            style: None,
            text: "Start".into(),
            data: b"launch".to_vec(),
        };

        assert!(callback_button_matches(&callback, "Start"));
        assert!(callback_button_matches(&callback, "launch"));
        assert!(callback_button_matches(&callback, "bGF1bmNo"));
        assert!(!callback_button_matches(&callback, "stop"));
    }

    #[test]
    fn find_callback_data_in_markup_returns_matching_payload() {
        let markup = tl::types::ReplyInlineMarkup {
            rows: vec![tl::types::KeyboardButtonRow {
                buttons: vec![
                    tl::types::KeyboardButtonUrl {
                        style: None,
                        text: "Docs".into(),
                        url: "https://example.com".into(),
                    }
                    .into(),
                    tl::types::KeyboardButtonCallback {
                        requires_password: false,
                        style: None,
                        text: "Start".into(),
                        data: b"launch".to_vec(),
                    }
                    .into(),
                ],
            }
            .into()],
        };

        let payload = find_callback_data_in_markup(Some(markup.into()), "Start").expect("payload");
        assert_eq!(payload, b"launch".to_vec());
    }

    #[test]
    fn actions_from_reply_markup_collects_inline_and_reply_keyboard_actions() {
        let inline_markup = tl::types::ReplyInlineMarkup {
            rows: vec![tl::types::KeyboardButtonRow {
                buttons: vec![
                    tl::types::KeyboardButtonCallback {
                        requires_password: false,
                        style: None,
                        text: "Start".into(),
                        data: b"launch".to_vec(),
                    }
                    .into(),
                    tl::types::KeyboardButtonUrl {
                        style: None,
                        text: "Docs".into(),
                        url: "https://example.com/docs".into(),
                    }
                    .into(),
                ],
            }
            .into()],
        };
        let reply_markup = tl::types::ReplyKeyboardMarkup {
            resize: false,
            single_use: false,
            selective: false,
            persistent: false,
            rows: vec![tl::types::KeyboardButtonRow {
                buttons: vec![
                    tl::types::KeyboardButton {
                        style: None,
                        text: "Yes".into(),
                    }
                    .into(),
                    tl::types::KeyboardButtonRequestPhone {
                        style: None,
                        text: "Share phone".into(),
                    }
                    .into(),
                ],
            }
            .into()],
            placeholder: None,
        };

        let inline_actions = actions_from_reply_markup(7, Some(inline_markup.into()));
        let reply_actions = actions_from_reply_markup(9, Some(reply_markup.into()));

        assert!(inline_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::InlineCallback
                && action.label == "Start"
                && action.callback_data_base64.as_deref() == Some("bGF1bmNo")
        }));
        assert!(inline_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::InlineUrl
                && action.url.as_deref() == Some("https://example.com/docs")
        }));
        assert!(reply_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::ReplyKeyboardText
                && action.trigger_text.as_deref() == Some("Yes")
        }));
        assert!(reply_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::ReplyKeyboardRequestPhone
                && !action.supported
        }));
    }

    #[test]
    fn actions_from_bot_info_collects_commands_and_menu_url() {
        let bot_info = tl::types::BotInfo {
            has_preview_medias: false,
            user_id: Some(42),
            description: Some("QA bot".into()),
            description_photo: None,
            description_document: None,
            commands: Some(vec![
                tl::types::BotCommand {
                    command: "start".into(),
                    description: "Start the bot".into(),
                }
                .into(),
                tl::types::BotCommand {
                    command: "help".into(),
                    description: "Show help".into(),
                }
                .into(),
            ]),
            menu_button: Some(
                tl::types::BotMenuButton {
                    text: "Open Web App".into(),
                    url: "https://example.com/app".into(),
                }
                .into(),
            ),
            privacy_policy_url: None,
            app_settings: None,
            verifier_settings: None,
        };
        let users = vec![tl::types::User {
            is_self: false,
            contact: false,
            mutual_contact: false,
            deleted: false,
            bot: true,
            bot_chat_history: false,
            bot_nochats: false,
            verified: false,
            restricted: false,
            min: false,
            bot_inline_geo: false,
            support: false,
            scam: false,
            apply_min_photo: false,
            fake: false,
            bot_attach_menu: false,
            premium: false,
            attach_menu_enabled: false,
            bot_can_edit: false,
            close_friend: false,
            stories_hidden: false,
            stories_unavailable: false,
            contact_require_premium: false,
            bot_business: false,
            bot_has_main_app: false,
            bot_forum_view: false,
            bot_forum_can_manage_topics: false,
            id: 42,
            access_hash: Some(99),
            first_name: Some("QA".into()),
            last_name: Some("Bot".into()),
            username: Some("qa_bot".into()),
            phone: None,
            photo: None,
            status: None,
            bot_info_version: None,
            restriction_reason: None,
            bot_inline_placeholder: None,
            lang_code: None,
            emoji_status: None,
            usernames: None,
            stories_max_id: None,
            color: None,
            profile_color: None,
            bot_active_users: None,
            bot_verification_icon: None,
            send_paid_messages_stars: None,
        }
        .into()];

        let direct_actions = actions_from_bot_info(&bot_info, &users, false);
        let group_actions = actions_from_bot_info(&bot_info, &users, true);

        assert!(direct_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::BotCommand
                && action.command.as_deref() == Some("/start")
        }));
        assert!(group_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::BotCommand
                && action.command.as_deref() == Some("/start@qa_bot")
        }));
        assert!(direct_actions.iter().any(|action| {
            action.action_kind == InteractiveActionKind::BotMenuUrl
                && action.url.as_deref() == Some("https://example.com/app")
        }));
    }
}
