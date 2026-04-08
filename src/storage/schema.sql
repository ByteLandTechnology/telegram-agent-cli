CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    login_state TEXT NOT NULL,
    api_id INTEGER,
    api_hash_ciphertext BLOB,
    api_hash_nonce BLOB,
    phone_ciphertext BLOB,
    phone_nonce BLOB,
    bot_token_ciphertext BLOB,
    bot_token_nonce BLOB,
    session_ciphertext BLOB,
    session_nonce BLOB,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    last_login_at TEXT
);

CREATE TABLE IF NOT EXISTS peer_aliases (
    alias TEXT PRIMARY KEY,
    peer_id INTEGER NOT NULL,
    peer_kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    username TEXT,
    packed_hex TEXT,
    last_resolved_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS test_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scenario_path TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS run_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    step_name TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(run_id) REFERENCES test_runs(id)
);
