//! HTTP server: axum router, SSE streaming, auth middleware, static UI.

use std::sync::Arc;

use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::engine::llm::LlmClient;
use crate::store::Store;

mod ask;
mod auth;
mod conversations;
mod static_ui;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub config: Config,
    pub llm: LlmClient,
}

/// Build the axum router and serve until the process is terminated.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let store = Arc::new(Store::open(std::path::Path::new(&config.database.path))?);
    seed_admin(&store, &config)?;

    let state = AppState {
        llm: LlmClient::new(&config.llm.base_url, &config.llm.model),
        config,
        store,
    };
    let bind = state.config.server.bind.clone();
    let app = build_router(Arc::new(state));

    let addr: std::net::SocketAddr = bind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address: {e}"))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn build_router(state: Arc<AppState>) -> axum::Router {
    let api = axum::Router::new()
        .route("/v1/auth/login", axum::routing::post(auth::login))
        .route("/v1/auth/logout", axum::routing::post(auth::logout))
        .route("/v1/me", axum::routing::get(auth::me))
        .route("/v1/ask", axum::routing::post(ask::ask))
        .route("/v1/conversations", axum::routing::get(conversations::list))
        .route(
            "/v1/conversations",
            axum::routing::post(conversations::create),
        )
        .route(
            "/v1/conversations/{id}",
            axum::routing::get(conversations::get).delete(conversations::delete),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_user,
        ));

    axum::Router::new()
        .nest("/v1", api)
        .fallback(static_ui::serve)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

/// Seed the first admin from config on first start (no-op if the user
/// already exists or no seed is configured).
fn seed_admin(store: &Store, config: &Config) -> anyhow::Result<()> {
    let Some(admin) = &config.admin else {
        return Ok(());
    };
    if store.get_user_by_username(&admin.username)?.is_some() {
        return Ok(());
    }
    store.create_user(&admin.username, &admin.password_hash, true)?;
    tracing::info!(username = %admin.username, "seeded admin user from config");
    Ok(())
}
