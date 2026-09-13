//! Conversation endpoints: list, create, get (with messages), delete.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::server::auth::UserIdentity;
use crate::server::AppState;

pub async fn list(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let store = state.store.clone();
    let uid = user.id;
    let convs = tokio::task::spawn_blocking(move || store.list_conversations(uid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        convs
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "title": c.title,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                })
            })
            .collect(),
    ))
}

#[derive(serde::Deserialize)]
pub struct CreateBody {
    pub title: Option<String>,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
    Json(body): Json<CreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let title = body.title.unwrap_or_else(|| "New conversation".into());
    let store = state.store.clone();
    let uid = user.id;
    let id = tokio::task::spawn_blocking(move || store.create_conversation(uid, &title))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.clone();
    let uid = user.id;
    let result = tokio::task::spawn_blocking(move || {
        let conv = store.get_conversation(&id, uid)?;
        let Some(conv) = conv else {
            return Err(anyhow::anyhow!("not found"));
        };
        let messages = store.recent_messages(&id, 10_000)?;
        Ok::<_, anyhow::Error>((conv, messages))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match result {
        Ok((conv, messages)) => Ok(Json(serde_json::json!({
            "id": conv.id,
            "title": conv.title,
            "created_at": conv.created_at,
            "updated_at": conv.updated_at,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role.as_str(),
                "content": m.content,
                "sources": m.sources,
            })).collect::<Vec<_>>(),
        }))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let store = state.store.clone();
    let uid = user.id;
    tokio::task::spawn_blocking(move || store.delete_conversation(&id, uid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
