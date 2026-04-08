use crate::errors::Result;
use crate::storage::{
    AccountProfile, AccountRecord, AliasRecord, EncryptedValue, NewAccount, RunEventRecord,
    SecretStore, TestRunRecord,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::str::FromStr;

pub struct AccountRepository {
    conn: Connection,
    secrets: SecretStore,
}

impl AccountRepository {
    pub fn open(path: impl AsRef<Path>, secrets: SecretStore) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(include_str!("schema.sql"))?;

        Ok(Self { conn, secrets })
    }

    pub fn insert_account(&self, account: NewAccount) -> Result<AccountRecord> {
        let encrypted_api_hash = self.secrets.encrypt_optional(account.api_hash.as_deref())?;
        let encrypted_phone = self.secrets.encrypt_optional(account.phone.as_deref())?;
        let encrypted_bot_token = self
            .secrets
            .encrypt_optional(account.bot_token.as_deref())?;
        let created_at = Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO accounts (
                name,
                kind,
                login_state,
                api_id,
                api_hash_ciphertext,
                api_hash_nonce,
                phone_ciphertext,
                phone_nonce,
                bot_token_ciphertext,
                bot_token_nonce,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                account.name,
                account.kind.as_str(),
                account.login_state.as_str(),
                account.api_id,
                encrypted_api_hash
                    .as_ref()
                    .map(|value| value.ciphertext.as_slice()),
                encrypted_api_hash
                    .as_ref()
                    .map(|value| value.nonce.as_slice()),
                encrypted_phone
                    .as_ref()
                    .map(|value| value.ciphertext.as_slice()),
                encrypted_phone.as_ref().map(|value| value.nonce.as_slice()),
                encrypted_bot_token
                    .as_ref()
                    .map(|value| value.ciphertext.as_slice()),
                encrypted_bot_token
                    .as_ref()
                    .map(|value| value.nonce.as_slice()),
                created_at,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        self.fetch_account(id)?.ok_or_else(|| {
            crate::errors::TelegramCliError::Message(
                "inserted account could not be reloaded".into(),
            )
        })
    }

    pub fn list_accounts(&self) -> Result<Vec<AccountRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                id,
                name,
                kind,
                login_state,
                is_default,
                api_id,
                phone_ciphertext,
                phone_nonce
            FROM accounts
            ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| self.map_account_row(row))?;
        let accounts = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(accounts)
    }

    pub fn set_default(&self, account_id: i64) -> Result<()> {
        let existing = self
            .conn
            .query_row(
                "SELECT id FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if existing.is_none() {
            return Err(crate::errors::TelegramCliError::Message(format!(
                "account {account_id} was not found"
            )));
        }

        self.conn.execute(
            "UPDATE accounts SET is_default = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
            params![account_id],
        )?;

        Ok(())
    }

    pub fn set_default_by_name(&self, name: &str) -> Result<()> {
        let account_id = self.find_account_id_by_name(name)?.ok_or_else(|| {
            crate::errors::TelegramCliError::Message(format!("account {name} was not found"))
        })?;
        self.set_default(account_id)
    }

    pub fn store_session(
        &self,
        name: &str,
        session: &str,
        login_state: crate::storage::LoginState,
    ) -> Result<()> {
        let encrypted_session = self.secrets.encrypt_optional(Some(session))?;
        let updated = self.conn.execute(
            r#"
            UPDATE accounts
            SET
                session_ciphertext = ?2,
                session_nonce = ?3,
                login_state = ?4,
                last_login_at = ?5
            WHERE name = ?1
            "#,
            params![
                name,
                encrypted_session
                    .as_ref()
                    .map(|value| value.ciphertext.as_slice()),
                encrypted_session
                    .as_ref()
                    .map(|value| value.nonce.as_slice()),
                login_state.as_str(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        if updated == 0 {
            return Err(crate::errors::TelegramCliError::Message(format!(
                "account {name} was not found"
            )));
        }

        Ok(())
    }

    pub fn clear_session(&self, name: &str) -> Result<()> {
        let updated = self.conn.execute(
            r#"
            UPDATE accounts
            SET
                session_ciphertext = NULL,
                session_nonce = NULL,
                login_state = 'pending',
                last_login_at = NULL
            WHERE name = ?1
            "#,
            params![name],
        )?;

        if updated == 0 {
            return Err(crate::errors::TelegramCliError::Message(format!(
                "account {name} was not found"
            )));
        }

        Ok(())
    }

    pub fn load_session(&self, name: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT session_ciphertext, session_nonce
                FROM accounts
                WHERE name = ?1
                "#,
                params![name],
                |row| {
                    let ciphertext = row.get::<_, Option<Vec<u8>>>(0)?;
                    let nonce = row.get::<_, Option<Vec<u8>>>(1)?;
                    Ok((ciphertext, nonce))
                },
            )
            .optional()?
            .map(|(ciphertext, nonce)| self.decrypt_field(ciphertext, nonce))
            .transpose()?
            .flatten())
    }

    pub fn find_account_by_name(&self, name: &str) -> Result<Option<AccountRecord>> {
        match self.find_account_id_by_name(name)? {
            Some(account_id) => self.fetch_account(account_id),
            None => Ok(None),
        }
    }

    pub fn find_default_account_name(&self) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT name FROM accounts WHERE is_default = 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn find_account_profile(&self, name: &str) -> Result<Option<AccountProfile>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT
                    id,
                    name,
                    kind,
                    login_state,
                    is_default,
                    api_id,
                    api_hash_ciphertext,
                    api_hash_nonce,
                    phone_ciphertext,
                    phone_nonce,
                    bot_token_ciphertext,
                    bot_token_nonce,
                    last_login_at
                FROM accounts
                WHERE name = ?1
                "#,
                params![name],
                |row| self.map_account_profile_row(row),
            )
            .optional()?)
    }

    pub fn mark_login_state(
        &self,
        name: &str,
        login_state: crate::storage::LoginState,
    ) -> Result<()> {
        let last_login_at = if login_state == crate::storage::LoginState::Authorized {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };
        let updated = self.conn.execute(
            r#"
            UPDATE accounts
            SET login_state = ?2, last_login_at = ?3
            WHERE name = ?1
            "#,
            params![name, login_state.as_str(), last_login_at],
        )?;

        if updated == 0 {
            return Err(crate::errors::TelegramCliError::Message(format!(
                "account {name} was not found"
            )));
        }

        Ok(())
    }

    pub fn upsert_alias(
        &self,
        alias: &str,
        peer_id: i64,
        peer_kind: crate::telegram::PeerKind,
        display_name: &str,
        username: Option<&str>,
        packed_hex: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO peer_aliases (alias, peer_id, peer_kind, display_name, username, packed_hex, last_resolved_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(alias) DO UPDATE SET
                peer_id = excluded.peer_id,
                peer_kind = excluded.peer_kind,
                display_name = excluded.display_name,
                username = excluded.username,
                packed_hex = excluded.packed_hex,
                last_resolved_at = excluded.last_resolved_at
            "#,
            params![
                alias,
                peer_id,
                peer_kind.as_str(),
                display_name,
                username,
                packed_hex,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn resolve_alias(&self, alias: &str) -> Result<Option<crate::telegram::ResolvedPeer>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT peer_id, peer_kind, display_name, username, packed_hex
                FROM peer_aliases
                WHERE alias = ?1
                "#,
                params![alias],
                |row| {
                    let peer_kind = row.get::<_, String>(1)?;
                    Ok(crate::telegram::ResolvedPeer {
                        peer_id: row.get(0)?,
                        peer_kind: crate::telegram::PeerKind::from_str(&peer_kind)
                            .map_err(to_sql_error)?,
                        display_name: row.get(2)?,
                        username: row.get(3)?,
                        packed_hex: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn list_aliases(&self) -> Result<Vec<AliasRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT alias, peer_id, peer_kind, display_name, username, packed_hex
            FROM peer_aliases
            ORDER BY alias ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let peer_kind = row.get::<_, String>(2)?;
            Ok(AliasRecord {
                alias: row.get(0)?,
                peer_id: row.get(1)?,
                peer_kind: crate::telegram::PeerKind::from_str(&peer_kind).map_err(to_sql_error)?,
                display_name: row.get(3)?,
                username: row.get(4)?,
                packed_hex: row.get(5)?,
            })
        })?;

        let aliases = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(aliases)
    }

    pub fn create_test_run(&self, scenario_path: &str) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO test_runs (scenario_path, started_at, status)
            VALUES (?1, ?2, 'running')
            "#,
            params![scenario_path, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn append_run_event(
        &self,
        run_id: i64,
        step_name: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO run_events (run_id, step_name, payload_json, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                run_id,
                step_name,
                serde_json::to_string(payload).map_err(|error| {
                    crate::errors::TelegramCliError::Message(format!(
                        "failed to serialize run event payload: {error}"
                    ))
                })?,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn finish_test_run(&self, run_id: i64, status: &str) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE test_runs
            SET status = ?2, finished_at = ?3
            WHERE id = ?1
            "#,
            params![run_id, status, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn latest_run(&self) -> Result<Option<TestRunRecord>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT id, scenario_path, status, started_at, finished_at
                FROM test_runs
                ORDER BY id DESC
                LIMIT 1
                "#,
                [],
                |row| {
                    Ok(TestRunRecord {
                        id: row.get(0)?,
                        scenario_path: row.get(1)?,
                        status: row.get(2)?,
                        started_at: row.get(3)?,
                        finished_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn find_run(&self, run_id: i64) -> Result<Option<TestRunRecord>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT id, scenario_path, status, started_at, finished_at
                FROM test_runs
                WHERE id = ?1
                "#,
                params![run_id],
                |row| {
                    Ok(TestRunRecord {
                        id: row.get(0)?,
                        scenario_path: row.get(1)?,
                        status: row.get(2)?,
                        started_at: row.get(3)?,
                        finished_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn list_run_events(&self, run_id: i64) -> Result<Vec<RunEventRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, step_name, payload_json, created_at
            FROM run_events
            WHERE run_id = ?1
            ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map(params![run_id], |row| {
            Ok(RunEventRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                step_name: row.get(2)?,
                payload_json: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let events = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(events)
    }

    fn fetch_account(&self, account_id: i64) -> Result<Option<AccountRecord>> {
        Ok(self
            .conn
            .query_row(
                r#"
                SELECT
                    id,
                    name,
                    kind,
                    login_state,
                    is_default,
                    api_id,
                    phone_ciphertext,
                    phone_nonce
                FROM accounts
                WHERE id = ?1
                "#,
                params![account_id],
                |row| self.map_account_row(row),
            )
            .optional()?)
    }

    fn find_account_id_by_name(&self, name: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM accounts WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn map_account_row(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
        let kind = row.get::<_, String>(2)?;
        let login_state = row.get::<_, String>(3)?;
        let phone = self
            .decrypt_field(
                row.get::<_, Option<Vec<u8>>>(6)?,
                row.get::<_, Option<Vec<u8>>>(7)?,
            )
            .map_err(to_sql_error)?;

        Ok(AccountRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: crate::storage::models::AccountKind::from_str(&kind).map_err(to_sql_error)?,
            login_state: crate::storage::models::LoginState::from_str(&login_state)
                .map_err(to_sql_error)?,
            is_default: row.get::<_, i64>(4)? != 0,
            api_id: row.get(5)?,
            phone,
        })
    }

    fn map_account_profile_row(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountProfile> {
        let kind = row.get::<_, String>(2)?;
        let login_state = row.get::<_, String>(3)?;
        let api_hash = self
            .decrypt_field(
                row.get::<_, Option<Vec<u8>>>(6)?,
                row.get::<_, Option<Vec<u8>>>(7)?,
            )
            .map_err(to_sql_error)?;
        let phone = self
            .decrypt_field(
                row.get::<_, Option<Vec<u8>>>(8)?,
                row.get::<_, Option<Vec<u8>>>(9)?,
            )
            .map_err(to_sql_error)?;
        let bot_token = self
            .decrypt_field(
                row.get::<_, Option<Vec<u8>>>(10)?,
                row.get::<_, Option<Vec<u8>>>(11)?,
            )
            .map_err(to_sql_error)?;

        Ok(AccountProfile {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: crate::storage::models::AccountKind::from_str(&kind).map_err(to_sql_error)?,
            login_state: crate::storage::models::LoginState::from_str(&login_state)
                .map_err(to_sql_error)?,
            is_default: row.get::<_, i64>(4)? != 0,
            api_id: row.get(5)?,
            api_hash,
            phone,
            bot_token,
            last_login_at: row.get(12)?,
        })
    }

    fn decrypt_field(
        &self,
        ciphertext: Option<Vec<u8>>,
        nonce: Option<Vec<u8>>,
    ) -> Result<Option<String>> {
        let encrypted = match (nonce, ciphertext) {
            (Some(nonce), Some(ciphertext)) => Some(EncryptedValue { nonce, ciphertext }),
            _ => None,
        };
        self.secrets.decrypt_optional(encrypted.as_ref())
    }
}

fn to_sql_error(error: crate::errors::TelegramCliError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
