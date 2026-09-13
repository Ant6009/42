//! SearXNG JSON API client.

use crate::engine::citations::Source;

/// Run a query against SearXNG and return the top results as sources.
pub async fn search(base_url: &str, query: &str, max: usize, snippet_chars: usize) -> anyhow::Result<Vec<Source>> {
    let _ = (base_url, query, max, snippet_chars);
    todo!("GET {base_url}/search?q=..&format=json, map to Source")
}
