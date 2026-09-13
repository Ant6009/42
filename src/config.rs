pub mod auth;
pub mod engine;
pub mod server;
pub mod store;
pub mod web;

/// 42 service configuration, loaded from a TOML file.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub llm: LlmConfig,
    pub search: SearchConfig,
    pub database: DatabaseConfig,
    pub admin: Option<AdminSeed>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    /// Listen address, e.g. "0.0.0.0:4242".
    pub bind: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmConfig {
    /// OpenAI-compatible base URL, e.g. "http://192.168.68.128:9292/v1".
    pub base_url: String,
    pub model: String,
    /// Context window size in tokens; oldest history is truncated first.
    #[serde(default = "default_context_window")]
    pub context_window: usize,
}

fn default_context_window() -> usize {
    32_768
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SearchConfig {
    /// SearXNG base URL with JSON API enabled.
    pub base_url: String,
    /// Number of results injected into the prompt per turn.
    #[serde(default = "default_max_sources")]
    pub max_sources: usize,
    /// Snippet length in characters.
    #[serde(default = "default_snippet_chars")]
    pub snippet_chars: usize,
}

fn default_max_sources() -> usize {
    8
}

fn default_snippet_chars() -> usize {
    500
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite file.
    pub path: String,
    /// Sliding window: how many recent turns are sent to the LLM.
    #[serde(default = "default_history_turns")]
    pub history_turns: usize,
}

fn default_history_turns() -> usize {
    10
}

/// First-admin bootstrap: seeded into SQLite on first start if absent.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminSeed {
    pub username: String,
    /// argon2id hash, produced by `42 admin hash-password`.
    pub password_hash: String,
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}
