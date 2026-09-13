//! Persistence: SQLite via rusqlite.
//!
//! Synchronous API; call sites in the async server wrap these in
//! `tokio::task::spawn_blocking`. One shared connection behind a Mutex is
//! fine for a LAN-scale service.

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};
use sha2::Digest;

pub mod types;
use types::{Conversation, Message, MessageRole, TokenKind, User};

use crate::engine::citations::Source;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_admin      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tokens (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN ('session', 'bearer')),
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL,
    expires_at   TEXT,
    last_used_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_tokens_user ON tokens(user_id);

CREATE TABLE IF NOT EXISTS conversations (
    id         TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_conv_user ON conversations(user_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS messages (
    id              INTEGER PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content         TEXT NOT NULL,
    sources         TEXT NOT NULL DEFAULT '[]',
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_msg_conv ON messages(conversation_id, id);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

/// All persistent state for 42.
pub struct Store {
    conn: Mutex<Connection>,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

impl Store {
    /// Open (or create) the database at `path` and ensure the schema exists.
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn with_conn<F: FnOnce(&mut Connection) -> anyhow::Result<T>, T>(
        &self,
        f: F,
    ) -> anyhow::Result<T> {
        let mut guard = self.conn.lock().expect("store poisoned");
        f(&mut guard)
    }

    // ------------------------------------------------------------- settings

    pub fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
            let out = match rows.next() {
                Some(Ok(v)) => Some(v),
                Some(Err(e)) => return Err(e.into()),
                None => None,
            };
            Ok(out)
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn get_all_settings(&self) -> anyhow::Result<std::collections::HashMap<String, String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut map = std::collections::HashMap::new();
            for row in rows {
                let (k, v) = row?;
                map.insert(k, v);
            }
            Ok(map)
        })
    }

    /// Insert default settings if the table is empty; no-op otherwise.
    pub fn seed_settings_if_empty(
        &self,
        defaults: &[(String, String)],
    ) -> anyhow::Result<bool> {
        self.with_conn(|conn| {
            let count: i64 =
                conn.query_row("SELECT count(*) FROM settings", [], |r| r.get(0))?;
            if count > 0 {
                return Ok(false);
            }
            for (k, v) in defaults {
                conn.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                    params![k, v],
                )?;
            }
            Ok(true)
        })
    }

    // ---------------------------------------------------------------- users

    pub fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        is_admin: bool,
    ) -> anyhow::Result<i64> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO users (username, password_hash, is_admin, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![username, password_hash, is_admin as i64, now()],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    pub fn get_user_by_username(&self, username: &str) -> anyhow::Result<Option<User>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, username, password_hash, is_admin, created_at
                 FROM users WHERE username = ?1",
            )?;
            let mut rows = stmt.query_map(params![username], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    is_admin: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                })
            })?;
            match rows.next() {
                Some(row) => Ok(Some(row?)),
                None => Ok(None),
            }
        })
    }

    // ---------------------------------------------------------------- tokens

    /// Create a token for `user_id`. Returns the raw token (shown once);
    /// only its SHA-256 hash is stored.
    pub fn create_token(
        &self,
        user_id: i64,
        kind: TokenKind,
        ttl: Option<std::time::Duration>,
    ) -> anyhow::Result<(i64, String)> {
        let bytes: [u8; 32] = rand::random();
        let mut raw = String::with_capacity(64);
        for b in &bytes {
            use std::fmt::Write as _;
            let _ = write!(raw, "{b:02x}");
        }
        let token_hash = format!("{:x}", sha2::Sha256::digest(raw.as_bytes()));
        let expires_at = ttl.map(|t| Utc::now() + chrono::TimeDelta::seconds(t.as_secs() as i64));
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tokens (user_id, kind, token_hash, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    user_id,
                    kind.as_str(),
                    token_hash,
                    now(),
                    expires_at.map(|t| t.to_rfc3339())
                ],
            )?;
            Ok((conn.last_insert_rowid(), raw))
        })
    }

    /// Look up a raw token; returns the owning user if valid and unexpired.
    pub fn validate_token(&self, raw: &str) -> anyhow::Result<Option<(TokenKind, User)>> {
        let token_hash = format!("{:x}", sha2::Sha256::digest(raw.as_bytes()));
        self.with_conn(|conn| {
            let user: Option<User> = conn
                .query_row(
                    "SELECT u.id, u.username, u.password_hash, u.is_admin, u.created_at
                     FROM tokens t JOIN users u ON u.id = t.user_id
                     WHERE t.token_hash = ?1",
                    params![token_hash],
                    |row| {
                        Ok(User {
                            id: row.get(0)?,
                            username: row.get(1)?,
                            password_hash: row.get(2)?,
                            is_admin: row.get::<_, i64>(3)? != 0,
                            created_at: row.get(4)?,
                        })
                    },
                )
                .ok();
            let Some(user) = user else {
                return Ok(None);
            };
            let (kind, expires_at): (String, Option<String>) = conn.query_row(
                "SELECT kind, expires_at FROM tokens WHERE token_hash = ?1",
                params![token_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if let Some(exp) = expires_at {
                if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&exp) {
                    if exp < Utc::now() {
                        return Ok(None);
                    }
                }
            }
            let _ = conn.execute(
                "UPDATE tokens SET last_used_at = ?1 WHERE token_hash = ?2",
                params![now(), token_hash],
            );
            Ok(Some((TokenKind::from_str_loose(&kind), user)))
        })
    }

    pub fn revoke_token(&self, raw: &str) -> anyhow::Result<()> {
        let token_hash = format!("{:x}", sha2::Sha256::digest(raw.as_bytes()));
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM tokens WHERE token_hash = ?1",
                params![token_hash],
            )?;
            Ok(())
        })
    }

    // -------------------------------------------------------- conversations

    pub fn create_conversation(&self, user_id: i64, title: &str) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, user_id, title, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, user_id, title, ts, ts],
            )?;
            Ok(id)
        })
    }

    pub fn list_conversations(&self, user_id: i64) -> anyhow::Result<Vec<Conversation>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, title, created_at, updated_at
                 FROM conversations WHERE user_id = ?1
                 ORDER BY updated_at DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![user_id], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// Fetch a conversation, verifying it belongs to `user_id`.
    pub fn get_conversation(&self, id: &str, user_id: i64) -> anyhow::Result<Option<Conversation>> {
        self.with_conn(|conn| {
            let conv: Option<Conversation> = conn
                .query_row(
                    "SELECT id, user_id, title, created_at, updated_at
                     FROM conversations WHERE id = ?1 AND user_id = ?2",
                    params![id, user_id],
                    |row| {
                        Ok(Conversation {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            title: row.get(2)?,
                            created_at: row.get(3)?,
                            updated_at: row.get(4)?,
                        })
                    },
                )
                .ok();
            Ok(conv)
        })
    }

    pub fn delete_conversation(&self, id: &str, user_id: i64) -> anyhow::Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM conversations WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )?;
            Ok(())
        })
    }

    // -------------------------------------------------------------- messages

    /// Append a message and bump the conversation's updated_at.
    pub fn append_message(
        &self,
        conversation_id: &str,
        role: MessageRole,
        content: &str,
        sources: &[Source],
    ) -> anyhow::Result<()> {
        let sources_json = serde_json::to_string(sources)?;
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (conversation_id, role, content, sources, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![conversation_id, role.as_str(), content, sources_json, now()],
            )?;
            conn.execute(
                "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
                params![now(), conversation_id],
            )?;
            Ok(())
        })
    }

    /// The `limit` most recent messages of a conversation, oldest first.
    pub fn recent_messages(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<Message>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT role, content, sources FROM (
                    SELECT role, content, sources, id
                    FROM messages WHERE conversation_id = ?1
                    ORDER BY id DESC LIMIT ?2
                 ) ORDER BY id ASC",
            )?;
            let rows = stmt.query_map(params![conversation_id, limit as i64], |row| {
                let sources_raw: String = row.get(2)?;
                let sources: Vec<Source> = serde_json::from_str(&sources_raw).unwrap_or_default();
                let role_raw: &str = &row.get::<_, String>(0)?;
                Ok(Message {
                    role: MessageRole::from_str_loose(role_raw),
                    content: row.get(1)?,
                    sources,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("test.sqlite")).unwrap();
        (dir, store)
    }

    #[test]
    fn user_lifecycle() {
        let (_dir, store) = temp_store();
        let id = store.create_user("alice", "hash", true).unwrap();
        let user = store.get_user_by_username("alice").unwrap().unwrap();
        assert_eq!(user.id, id);
        assert!(user.is_admin);
        assert!(store.get_user_by_username("bob").unwrap().is_none());
    }

    #[test]
    fn token_roundtrip_and_expiry() {
        let (_dir, store) = temp_store();
        let uid = store.create_user("alice", "hash", false).unwrap();
        let (_id, raw) = store
            .create_token(
                uid,
                TokenKind::Session,
                Some(std::time::Duration::from_secs(3600)),
            )
            .unwrap();
        let (kind, user) = store.validate_token(&raw).unwrap().unwrap();
        assert_eq!(kind, TokenKind::Session);
        assert_eq!(user.username, "alice");
        store.revoke_token(&raw).unwrap();
        assert!(store.validate_token(&raw).unwrap().is_none());
    }

    #[test]
    fn conversation_and_messages() {
        let (_dir, store) = temp_store();
        let uid = store.create_user("alice", "hash", false).unwrap();
        let cid = store.create_conversation(uid, "first").unwrap();

        let sources = vec![Source {
            index: 1,
            title: "Example".into(),
            url: "https://example.com".into(),
            snippet: "snippet".into(),
        }];
        store
            .append_message(&cid, MessageRole::User, "hello", &[])
            .unwrap();
        store
            .append_message(&cid, MessageRole::Assistant, "hi there", &sources)
            .unwrap();

        let msgs = store.recent_messages(&cid, 10).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, MessageRole::User);
        assert_eq!(msgs[1].sources.len(), 1);
        assert_eq!(msgs[1].sources[0].url, "https://example.com");

        // Windowing: limit 1 returns only the newest.
        let msgs = store.recent_messages(&cid, 1).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, MessageRole::Assistant);

        // Ownership: another user cannot see or delete it.
        let other = store.create_user("bob", "hash", false).unwrap();
        assert!(store.get_conversation(&cid, other).unwrap().is_none());
        store.delete_conversation(&cid, other).unwrap();
        assert!(store.get_conversation(&cid, uid).unwrap().is_some());
        store.delete_conversation(&cid, uid).unwrap();
        let list = store.list_conversations(uid).unwrap();
        assert!(list.is_empty());
    }
}
