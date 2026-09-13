//! HTTP server: axum router, SSE streaming, auth middleware, static UI.

use crate::config::Config;

/// Build the axum router and serve until the process is signaled.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let _ = config;
    todo!("router: / (static UI), /v1/ask (SSE), /v1/conversations, /v1/auth/*; seed admin from config; start tokio server")
}
