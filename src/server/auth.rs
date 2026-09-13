//! Auth endpoints and the session middleware.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::TimeDelta;

use crate::auth::AuthService;
use crate::server::AppState;
use crate::store::types::User;

pub const SESSION_COOKIE: &str = "42_session";
const SESSION_TTL: TimeDelta = TimeDelta::days(30);

#[derive(serde::Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Response, (StatusCode, String)> {
    let store = state.store.clone();
    let user = tokio::task::spawn_blocking(move || {
        AuthService::authenticate(&store, &body.username, &body.password)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let Some(user) = user else {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    };

    let store = state.store.clone();
    let raw = tokio::task::spawn_blocking(move || {
        AuthService::create_session_token(
            &store,
            user.id,
            std::time::Duration::from_secs(SESSION_TTL.num_seconds() as u64),
        )
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut response = (
        StatusCode::OK,
        Json(serde_json::json!({ "username": user.username, "is_admin": user.is_admin })),
    )
        .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        format!(
            "{SESSION_COOKIE}={raw}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
            SESSION_TTL.num_seconds()
        )
        .parse()
        .unwrap(),
    );
    Ok(response)
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(cookie) = cookie_from(&headers) {
        let store = state.store.clone();
        let _ = tokio::task::spawn_blocking(move || store.revoke_token(&cookie)).await;
    }
    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn me(UserIdentity(user): UserIdentity) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "username": user.username,
        "is_admin": user.is_admin,
    }))
}

/// Extracted, authenticated user for downstream handlers.
#[derive(Clone)]
pub struct UserIdentity(pub User);

/// Reject requests without a valid session cookie.
pub async fn require_user(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers().clone();
    let Some(raw) = cookie_from(&headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let store = state.store.clone();
    let user = tokio::task::spawn_blocking(move || store.validate_token(&raw))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((_kind, user)) = user else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    req.extensions_mut().insert(UserIdentity(user.clone()));
    Ok(next.run(req).await)
}

fn cookie_from(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .map(|s| s.trim())
        .find_map(|pair| pair.strip_prefix(&format!("{SESSION_COOKIE}=")))
        .map(|s| s.to_string())
}

/// Axum extractor: pulls the UserIdentity out of request extensions.
impl<S> axum::extract::FromRequestParts<S> for UserIdentity
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserIdentity>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "missing user identity (middleware not applied?)".into(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_parsing() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "a=1; 42_session=abc123; b=2".parse().unwrap(),
        );
        assert_eq!(cookie_from(&headers).as_deref(), Some("abc123"));

        let mut headers = axum::http::HeaderMap::new();
        headers.insert(header::COOKIE, "a=1".parse().unwrap());
        assert!(cookie_from(&headers).is_none());

        let empty = axum::http::HeaderMap::new();
        assert!(cookie_from(&empty).is_none());
    }
}
