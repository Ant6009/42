//! Citation marker validation and repair.
//!
//! The LLM is instructed to cite sources as `[1]`, `[2]`, ... The validator
//! drops or repairs any marker that does not reference an actual source, so
//! weak local models cannot invent citations.

/// A source returned by search and injected into the prompt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Source {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Remove citation markers that do not reference a valid source index.
pub fn validate(text: &str, source_count: usize) -> String {
    let _ = (text, source_count);
    todo!("parse [n] markers, drop/repair invalid ones")
}
