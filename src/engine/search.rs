//! SearXNG JSON API client.

use crate::engine::citations::Source;

/// Run a query against SearXNG and return the top results as sources.
pub async fn search(
    base_url: &str,
    query: &str,
    max: usize,
    snippet_chars: usize,
) -> anyhow::Result<Vec<Source>> {
    let url = format!("{base_url}/search");
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .query(&[("q", query), ("format", "json")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_results(&body, max, snippet_chars))
}

/// Map a SearXNG JSON response to numbered sources. Pure, so unit-testable.
pub fn parse_results(body: &serde_json::Value, max: usize, snippet_chars: usize) -> Vec<Source> {
    let Some(results) = body.get("results").and_then(|r| r.as_array()) else {
        return Vec::new();
    };
    results
        .iter()
        .take(max)
        .filter_map(|r| {
            let url = r.get("url")?.as_str()?.to_string();
            let title = r
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or(&url)
                .to_string();
            let snippet = r
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .chars()
                .take(snippet_chars)
                .collect();
            Some(Source {
                index: 0, // assigned below
                title,
                url,
                snippet,
            })
        })
        .enumerate()
        .map(|(i, mut s)| {
            s.index = i + 1;
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(json: &str) -> serde_json::Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn maps_results_with_indices() {
        let v = body(
            r#"{"results": [
                {"url": "https://a.example", "title": "A", "content": "alpha"},
                {"url": "https://b.example", "title": "B", "content": "beta"}
            ]}"#,
        );
        let sources = parse_results(&v, 8, 500);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].index, 1);
        assert_eq!(sources[1].index, 2);
        assert_eq!(sources[1].url, "https://b.example");
    }

    #[test]
    fn respects_max_and_truncates_snippets() {
        let v = body(
            r#"{"results": [
                {"url": "https://a.example", "title": "A", "content": "0123456789"},
                {"url": "https://b.example", "title": "B", "content": "xy"},
                {"url": "https://c.example", "title": "C", "content": "z"}
            ]}"#,
        );
        let sources = parse_results(&v, 2, 4);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].snippet, "0123");
    }

    #[test]
    fn tolerates_missing_fields() {
        let v = body(r#"{"results": [{"url": "https://a.example"}]}"#);
        let sources = parse_results(&v, 8, 500);
        assert_eq!(sources.len(), 1);
        // Title falls back to the URL; snippet is empty.
        assert_eq!(sources[0].title, "https://a.example");
        assert_eq!(sources[0].snippet, "");
    }

    #[test]
    fn empty_or_malformed_response() {
        assert!(parse_results(&body("{}"), 8, 500).is_empty());
        assert!(parse_results(&body(r#"{"results": "nope"}"#), 8, 500).is_empty());
    }
}
