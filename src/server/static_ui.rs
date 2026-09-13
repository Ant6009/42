//! Serve the embedded static UI (no build step; see src/web/).

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

use crate::web;

pub async fn serve(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path();
    match path {
        "/" | "/index.html" => html(web::INDEX_HTML),
        "/style.css" => css(web::STYLE_CSS),
        "/app.js" => js(web::APP_JS),
        _ => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

fn html(body: &str) -> axum::response::Response {
    response(body, "text/html; charset=utf-8")
}
fn css(body: &str) -> axum::response::Response {
    response(body, "text/css; charset=utf-8")
}
fn js(body: &str) -> axum::response::Response {
    response(body, "text/javascript; charset=utf-8")
}

fn response(body: &str, content_type: &str) -> axum::response::Response {
    let mut resp = axum::response::Response::new(axum::body::Body::from(body.to_string()));
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    // The UI is embedded in the binary; every deploy changes it, so never
    // let browsers heuristic-cache it.
    h.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    *resp.status_mut() = StatusCode::OK;
    resp
}
