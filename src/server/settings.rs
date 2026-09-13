//! Admin-only engine settings: GET/PUT /v1/settings.
//!
//! Global, runtime-tunable (LLM endpoint/model, SearXNG, budgets). Kept
//! dumb: values are stored as-is; misconfiguration surfaces as an SSE
//! error on the next ask.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::config::EngineSettings;
use crate::server::auth::UserIdentity;
use crate::server::AppState;

fn require_admin(user: &crate::store::types::User) -> Result<(), (StatusCode, String)> {
    if user.is_admin {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "admin only".into()))
    }
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
) -> Result<Json<EngineSettings>, (StatusCode, String)> {
    require_admin(&user)?;
    let store = state.store.clone();
    let config = state.config.clone();
    let settings = tokio::task::spawn_blocking(move || -> anyhow::Result<EngineSettings> {
        let map = store.get_all_settings()?;
        Ok(EngineSettings::from_map(&map, &config))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(settings))
}

pub async fn put_settings(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
    Json(body): Json<EngineSettings>,
) -> Result<Json<EngineSettings>, (StatusCode, String)> {
    require_admin(&user)?;
    let store = state.store.clone();
    let pairs = body.to_pairs();
    let result =
        tokio::task::spawn_blocking(move || -> anyhow::Result<std::collections::HashMap<String, String>> {
            for (k, v) in &pairs {
                store.set_setting(k, v)?;
            }
            store.get_all_settings()
        })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tracing::info!("engine settings updated by admin");
    Ok(Json(EngineSettings::from_map(&result, &state.config)))
}
