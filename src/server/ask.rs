//! POST /v1/ask — the core endpoint.
//!
//! Flow: authenticate (middleware) -> resolve/create conversation ->
//! persist user message -> SearXNG search -> build prompt -> stream LLM
//! tokens as SSE -> validate citations -> persist assistant message.
//!
//! SSE events (JSON payloads):
//!   {"type":"conversation","id":"..."}
//!   {"type":"sources","sources":[...]}
//!   {"type":"token","text":"..."}
//!   {"type":"final","text":"..."}   (only if citation validation changed text)
//!   {"type":"done"}
//!   {"type":"error","message":"..."}

use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{Stream, StreamExt};

use crate::config::EngineSettings;
use crate::engine::citations;
use crate::engine::llm::{LlmClient, LlmDelta};
use crate::engine::prompt::{build_messages, PromptOptions};
use crate::engine::search;
use crate::server::auth::UserIdentity;
use crate::server::AppState;
use crate::store::types::MessageRole;

#[derive(serde::Deserialize)]
pub struct AskBody {
    /// Existing conversation to continue; omit to start a new one.
    pub conversation_id: Option<String>,
    pub question: String,
}

pub async fn ask(
    State(state): State<Arc<AppState>>,
    UserIdentity(user): UserIdentity,
    Json(body): Json<AskBody>,
) -> impl IntoResponse {
    let question = body.question.trim().to_string();
    let stream = ask_stream(state, user, question, body.conversation_id).await;
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn ask_stream(
    state: Arc<AppState>,
    user: crate::store::types::User,
    question: String,
    conversation_id: Option<String>,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        use std::convert::Infallible;

        let ev = |v: serde_json::Value| Ok(Event::default().json_data(v).unwrap());

        // 0. Load current engine settings (admin may have changed them).
        let store = state.store.clone();
        let config = state.config.clone();
        let engine = match tokio::task::spawn_blocking(move || {
            let map = store.get_all_settings()?;
            Ok::<_, anyhow::Error>(EngineSettings::from_map(&map, &config))
        })
        .await
        .map_err(|e| -> Infallible { unreachable!("{e:?}") })?
        {
            Ok(s) => s,
            Err(e) => {
                yield ev(serde_json::json!({"type": "error", "message": e.to_string()}));
                return;
            }
        };

        // 1. Resolve or create the conversation.
        let store = state.store.clone();
        let cid = match conversation_id {
            Some(id) => {
                let id_check = id.clone();
                match tokio::task::spawn_blocking(move || store.get_conversation(&id_check, user.id)).await
                    .map_err(|e| -> Infallible { unreachable!("{e:?}") })?
                {
                    Ok(Some(_)) => id,
                    _ => {
                        yield ev(serde_json::json!({"type": "error", "message": "conversation not found"}));
                        return;
                    }
                }
            }
            None => {
                let title: String = question.chars().take(60).collect();
                match tokio::task::spawn_blocking(move || store.create_conversation(user.id, &title))
                    .await
                    .map_err(|e| -> Infallible { unreachable!("{e:?}") })?
                {
                    Ok(id) => {
                        yield ev(serde_json::json!({"type": "conversation", "id": id.clone()}));
                        id
                    }
                    Err(e) => {
                        yield ev(serde_json::json!({"type": "error", "message": e.to_string()}));
                        return;
                    }
                }
            }
        };

        // 2. Persist the user message and load history (without it).
        let store = state.store.clone();
        let cid_hist = cid.clone();
        let q_hist = question.clone();
        let history_turns = engine.history_window;
        let history = match tokio::task::spawn_blocking(move || {
            store.append_message(&cid_hist, MessageRole::User, &q_hist, &[])?;
            store.recent_messages(&cid_hist, history_turns)
        })
        .await
        .map_err(|e| -> Infallible { unreachable!("{e:?}") })?
        {
            Ok(h) => h,
            Err(e) => {
                yield ev(serde_json::json!({"type": "error", "message": e.to_string()}));
                return;
            }
        };
        // Drop the user message we just appended; it becomes the question.
        let history: Vec<_> = history.into_iter().filter(|m| m.role != MessageRole::User || m.content != question).collect();

        // 3. Search.
        let sources = match search::search(
            &engine.search_url,
            &question,
            engine.search_max_results,
            engine.search_snippet_chars,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "search failed; answering without sources");
                Vec::new()
            }
        };
        yield ev(serde_json::json!({"type": "sources", "sources": sources}));

        // 4. Build the prompt and stream the LLM.
        let opts = PromptOptions {
            context_window: engine.llm_context_window,
            history_turns: engine.history_window,
        };
        let messages = build_messages(&history, &question, &sources, opts);
        let mut answer = String::new();
        let llm = LlmClient::with_http(
            state.http.clone(),
            &engine.llm_base_url,
            &engine.llm_model,
        );
        let mut llm = match llm.stream_chat(&messages) {
            Ok(s) => s,
            Err(e) => {
                yield ev(serde_json::json!({"type": "error", "message": e.to_string()}));
                return;
            }
        };
        while let Some(delta) = llm.next().await {
            match delta {
                Ok(LlmDelta::Token(t)) => {
                    answer.push_str(&t);
                    yield ev(serde_json::json!({"type": "token", "text": t}));
                }
                Ok(LlmDelta::Thinking(t)) => {
                    // Reasoning phase: not part of the answer, but signals
                    // progress so the UI can show a Thinking indicator.
                    yield ev(serde_json::json!({"type": "thinking", "text": t}));
                }
                Err(e) => {
                    yield ev(serde_json::json!({"type": "error", "message": e.to_string()}));
                    return;
                }
            }
        }

        // 5. Validate citations; persist the assistant message.
        let validated = citations::validate(&answer, sources.len());
        let store = state.store.clone();
        let src = sources.clone();
        let cid2 = cid.clone();
        let validated_persist = validated.clone();
        let persisted = tokio::task::spawn_blocking(move || {
            store.append_message(&cid2, MessageRole::Assistant, &validated_persist, &src)
        })
        .await
        .map_err(|e| -> Infallible { unreachable!("{e:?}") })?;
        if let Err(e) = persisted {
            tracing::error!(error = %e, "failed to persist assistant message");
        }
        if validated != answer {
            yield ev(serde_json::json!({"type": "final", "text": validated}));
        }
        yield ev(serde_json::json!({"type": "done"}));
    }
}
