//! Persistence: SQLite via rusqlite (synchronous, called through
//! spawn_blocking at the call sites).

/// Storage trait so the engine stays decoupled from SQLite details.
pub trait ConversationStore: Send + Sync {
    fn append_turn(&self) -> anyhow::Result<()>;
    fn recent_turns(&self) -> anyhow::Result<Vec<String>>;
}

pub struct SqliteStore;
