//! Streaming chat completions client for OpenAI-compatible endpoints
//! (llama.cpp server, Ollama, LM Studio, ...).

use std::pin::Pin;

use futures_util::{Stream, StreamExt};

/// Client bound to one base URL and model.
#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Stream chat completions; yields content tokens as they arrive.
    pub fn stream_chat(
        &self,
        messages: &[serde_json::Value],
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        let url = format!("{}/chat/completions", self.base_url);
        let http = self.http.clone();
        let model = self.model.clone();
        let body_messages = messages.to_vec();
        Ok(async_stream::stream! {
            let body = serde_json::json!({
                "model": model,
                "messages": body_messages,
                "stream": true,
            });
            let response = match http.post(&url).json(&body).send().await {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => r,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("LLM request failed: {e}"));
                        return;
                    }
                },
                Err(e) => {
                    yield Err(anyhow::anyhow!("LLM request failed: {e}"));
                    return;
                }
            };

            // Buffer bytes and split on newlines; SSE events are line-based.
            let mut buf: Vec<u8> = Vec::new();
            let mut chunks = response.bytes_stream();
            while let Some(chunk) = chunks.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("LLM stream error: {e}"));
                        return;
                    }
                };
                buf.extend_from_slice(&chunk);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
                    let line = String::from_utf8_lossy(&line_bytes);
                    if let Some(token) = extract_token(&line) {
                        yield Ok(token);
                    }
                }
            }
            // Trailing line without a final newline.
            if !buf.is_empty() {
                let line = String::from_utf8_lossy(&buf);
                if let Some(token) = extract_token(&line) {
                    yield Ok(token);
                }
            }
        }
        .boxed())
    }
}

/// Extract the content token from one SSE line, if any.
///
/// Lines look like `data: {"choices":[{"delta":{"content":"..."}}]}`;
/// the stream ends with `data: [DONE]`.
pub fn extract_token(line: &str) -> Option<String> {
    let payload = line.trim().strip_prefix("data:")?.trim();
    if payload == "[DONE]" {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(payload).ok()?;
    v.pointer("/choices/0/delta/content")?
        .as_str()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streams_tokens_from_mock_server() {
        use axum::routing::post;
        use axum::Router;

        let sse_body = "\
data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}

data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}

data: [DONE]
";
        let app = Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let body = sse_body.to_string();
                async move {
                    axum::response::Response::builder()
                        .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
                        .body(body)
                        .unwrap()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = LlmClient::new(&format!("http://{addr}/v1"), "test-model");
        let messages = vec![serde_json::json!({"role": "user", "content": "hi"})];
        let mut stream = client.stream_chat(&messages).unwrap();
        let mut collected = String::new();
        while let Some(token) = stream.next().await {
            collected.push_str(&token.unwrap());
        }
        assert_eq!(collected, "Hello");
    }

    #[test]
    fn extract_token_parses_sse_lines() {
        assert_eq!(
            extract_token(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#),
            Some("hi".into())
        );
        assert_eq!(extract_token("data: [DONE]"), None);
        assert_eq!(extract_token(""), None);
        assert_eq!(extract_token(": keepalive comment"), None);
        // Empty delta (role-only chunk) yields no token.
        assert_eq!(
            extract_token(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
    }
}
