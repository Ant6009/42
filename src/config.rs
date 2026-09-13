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

/// Runtime-tunable engine settings.
///
/// Seeded into the `settings` table from TOML on first start; afterwards the
/// database is the source of truth and admins can change these from the UI
/// without a restart. TOML keeps only what cannot change at runtime
/// (`server.bind`, `db.path`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineSettings {
    pub llm_base_url: String,
    pub llm_model: String,
    pub llm_context_window: usize,
    pub search_url: String,
    pub search_max_results: usize,
    pub search_snippet_chars: usize,
    pub history_window: usize,
}

impl EngineSettings {
    pub fn from_config(c: &Config) -> Self {
        Self {
            llm_base_url: c.llm.base_url.clone(),
            llm_model: c.llm.model.clone(),
            llm_context_window: c.llm.context_window,
            search_url: c.search.base_url.clone(),
            search_max_results: c.search.max_sources,
            search_snippet_chars: c.search.snippet_chars,
            history_window: c.database.history_turns,
        }
    }

    /// Settings as (key, value) pairs for the settings table.
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        vec![
            ("llm.base_url".into(), self.llm_base_url.clone()),
            ("llm.model".into(), self.llm_model.clone()),
            (
                "llm.context_window".into(),
                self.llm_context_window.to_string(),
            ),
            ("search.url".into(), self.search_url.clone()),
            (
                "search.max_results".into(),
                self.search_max_results.to_string(),
            ),
            (
                "search.snippet_chars".into(),
                self.search_snippet_chars.to_string(),
            ),
            (
                "conversation.history_window".into(),
                self.history_window.to_string(),
            ),
        ]
    }

    /// Build from a settings map, falling back to TOML defaults for any
    /// missing or malformed key.
    pub fn from_map(
        map: &std::collections::HashMap<String, String>,
        fallback: &Config,
    ) -> Self {
        let base = Self::from_config(fallback);
        let get = |k: &str, default: String| {
            map.get(k).cloned().unwrap_or(default)
        };
        let get_us = |k: &str, default: usize| {
            map.get(k)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
                .max(1)
        };
        Self {
            llm_base_url: get("llm.base_url", base.llm_base_url),
            llm_model: get("llm.model", base.llm_model),
            llm_context_window: get_us("llm.context_window", base.llm_context_window),
            search_url: get("search.url", base.search_url),
            search_max_results: get_us("search.max_results", base.search_max_results),
            search_snippet_chars: get_us("search.snippet_chars", base.search_snippet_chars),
            history_window: get_us("conversation.history_window", base.history_window),
        }
    }
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
