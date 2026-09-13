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

/// Remove or repair citation markers that reference invalid sources.
///
/// A bracket group is treated as a citation only when its content is
/// exclusively comma-separated numbers (e.g. `[1]`, `[2,4]`). Anything else
/// (markdown links, prose) is left untouched. Within a citation, valid
/// indices are kept and invalid ones dropped; a citation with no valid
/// indices is removed entirely.
pub fn validate(text: &str, source_count: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close) = find_closing(&chars, i) {
                let inner: String = chars[i + 1..close].iter().collect();
                let tokens: Vec<&str> = inner
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .collect();
                let all_numeric = !tokens.is_empty()
                    && tokens.iter().all(|t| t.bytes().all(|b| b.is_ascii_digit()));
                if all_numeric {
                    let valid: Vec<&str> = tokens
                        .iter()
                        .filter(|t| {
                            t.parse::<usize>()
                                .is_ok_and(|n| (1..=source_count).contains(&n))
                        })
                        .copied()
                        .collect();
                    if valid.is_empty() {
                        i = close + 1;
                        continue;
                    }
                    out.push('[');
                    out.push_str(&valid.join(","));
                    out.push(']');
                    i = close + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Index of the `]` closing the `[` at `open`, or None. No nesting support:
/// citation markers never contain brackets.
fn find_closing(chars: &[char], open: usize) -> Option<usize> {
    chars[open + 1..]
        .iter()
        .position(|&c| c == ']')
        .map(|p| open + 1 + p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_markers() {
        assert_eq!(validate("Foo [1] bar [2]", 2), "Foo [1] bar [2]");
        assert_eq!(validate("[1][2]", 2), "[1][2]");
        assert_eq!(validate("multi [1,2] end", 2), "multi [1,2] end");
    }

    #[test]
    fn drops_invalid_markers() {
        assert_eq!(validate("Foo [9] bar", 2), "Foo  bar");
        assert_eq!(validate("[3]", 2), "");
    }

    #[test]
    fn repairs_mixed_markers() {
        assert_eq!(validate("see [1,9] for details", 2), "see [1] for details");
    }

    #[test]
    fn leaves_non_citations_alone() {
        let md = "[see here](https://example.com) and [brackets] in prose";
        assert_eq!(validate(md, 2), md);
    }

    #[test]
    fn handles_zero_sources() {
        assert_eq!(validate("no [1] sources at all", 0), "no  sources at all");
    }
}
