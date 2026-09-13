//! Prompt assembly: system prompt + sources + sliding-window history,
//! bounded by the configured context window (oldest turns truncated first).

/// Build the chat message list for one LLM call.
pub fn build_messages() -> Vec<serde_json::Value> {
    todo!("system prompt with citation instructions, sources block, history window, current question")
}
