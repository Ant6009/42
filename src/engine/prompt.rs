//! Prompt assembly: system prompt + sources + sliding-window history,
//! bounded by the configured context window (oldest messages dropped first).

use crate::engine::citations::Source;
use crate::store::types::Message;

const SYSTEM_PROMPT: &str = "\
You are 42, a search-augmented assistant. Answer the user's question using \
the web sources provided below. Cite sources inline with bracketed numbers \
that match the source numbers, like [1] or [2,3]. Only cite sources that are \
actually listed. Be concise and factual. If the sources do not contain the \
answer, say so plainly instead of guessing.";

/// Options controlling how the prompt is bounded.
#[derive(Debug, Clone, Copy)]
pub struct PromptOptions {
    /// Total context budget in tokens (rough estimate: 4 chars/token).
    pub context_window: usize,
    /// Maximum number of history messages included.
    pub history_turns: usize,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            context_window: 32_768,
            history_turns: 10,
        }
    }
}

/// Build the OpenAI-format chat message list for one LLM call.
///
/// `history` is the prior conversation (oldest first, already windowed by
/// the caller). The current `question` is appended last. History messages
/// are dropped from the oldest end until the whole prompt fits the context
/// budget; the system message, sources, and current question always fit.
pub fn build_messages(
    history: &[Message],
    question: &str,
    sources: &[Source],
    opts: PromptOptions,
) -> Vec<serde_json::Value> {
    let system = system_prompt(sources);
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": system,
    })];

    let fixed_tokens = estimate_tokens(&system) + estimate_tokens(question);
    let mut budget = opts.context_window.saturating_sub(fixed_tokens);

    // Take the most recent history_turns, then drop from the oldest end
    // while over budget.
    let start = history.len().saturating_sub(opts.history_turns);
    let recent = &history[start..];
    let mut kept: Vec<&Message> = Vec::new();
    for msg in recent.iter().rev() {
        let cost = estimate_tokens(&msg.content);
        if cost > budget {
            break;
        }
        budget -= cost;
        kept.push(msg);
    }
    kept.reverse();
    for msg in kept {
        messages.push(serde_json::json!({
            "role": msg.role.as_str(),
            "content": msg.content,
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": question,
    }));
    messages
}

/// System prompt with the numbered sources block.
pub fn system_prompt(sources: &[Source]) -> String {
    let mut out = String::with_capacity(256 + sources.len() * 128);
    out.push_str(SYSTEM_PROMPT);
    if sources.is_empty() {
        out.push_str("\n\nNo web sources were found for this question.");
        return out;
    }
    out.push_str("\n\nWeb sources:");
    for s in sources {
        out.push_str(&format!(
            "\n[{}] {}\n{}\n{}",
            s.index, s.title, s.url, s.snippet
        ));
    }
    out
}

/// Rough token estimate: 4 characters per token. Deliberately crude; local
/// models vary, and we only need a safe upper bound.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::types::MessageRole;

    fn msg(role: MessageRole, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            sources: Vec::new(),
        }
    }

    fn source(i: usize) -> Source {
        Source {
            index: i,
            title: format!("Title {i}"),
            url: format!("https://example.com/{i}"),
            snippet: "snippet".into(),
        }
    }

    #[test]
    fn basic_shape() {
        let history = vec![
            msg(MessageRole::User, "first question"),
            msg(MessageRole::Assistant, "first answer"),
        ];
        let msgs = build_messages(
            &history,
            "second question",
            &[source(1), source(2)],
            PromptOptions::default(),
        );
        let roles: Vec<&str> = msgs.iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, vec!["system", "user", "assistant", "user"]);
        let system = msgs[0]["content"].as_str().unwrap();
        assert!(system.contains("[1] Title 1"));
        assert!(system.contains("https://example.com/2"));
        assert_eq!(msgs[3]["content"].as_str().unwrap(), "second question");
    }

    #[test]
    fn empty_sources_noted() {
        let system = system_prompt(&[]);
        assert!(system.contains("No web sources were found"));
    }

    #[test]
    fn history_window_limits_messages() {
        // 6 history messages, window of 4: only the last 4 survive.
        let history: Vec<Message> = (1..=6)
            .map(|i| {
                if i % 2 == 1 {
                    msg(MessageRole::User, &format!("q{i}"))
                } else {
                    msg(MessageRole::Assistant, &format!("a{i}"))
                }
            })
            .collect();
        let opts = PromptOptions {
            context_window: 100_000,
            history_turns: 4,
        };
        let msgs = build_messages(&history, "final", &[], opts);
        // system + 4 history + question
        assert_eq!(msgs.len(), 6);
        assert_eq!(msgs[1]["content"].as_str().unwrap(), "q3");
    }

    #[test]
    fn context_budget_drops_oldest_history() {
        let big = "x".repeat(8000); // ~2000 tokens each
        let history = vec![
            msg(MessageRole::User, &big),
            msg(MessageRole::Assistant, &big),
            msg(MessageRole::User, "small"),
        ];
        // Budget fits system + question + one big + small, but not two bigs:
        // the oldest big message must be dropped.
        let opts = PromptOptions {
            context_window: 4100,
            history_turns: 10,
        };
        let msgs = build_messages(&history, "q", &[], opts);
        let contents: Vec<&str> = msgs
            .iter()
            .skip(1)
            .map(|m| m["content"].as_str().unwrap())
            .collect();
        let big_count = contents.iter().filter(|c| **c == big).count();
        assert_eq!(big_count, 1, "exactly one big message should survive");
        assert!(contents.contains(&"small"));
        assert_eq!(*contents.last().unwrap(), "q");
    }
}
