//! Embedded static UI (vanilla HTML/CSS/JS, no build step).
//!
//! Files are included at compile time via include_str! and served by
//! tower-http. See DESIGN.md: prompt box, streaming markdown answer,
//! sources panel, conversation sidebar, login page.

pub const INDEX_HTML: &str = include_str!("index.html");
pub const STYLE_CSS: &str = include_str!("style.css");
pub const APP_JS: &str = include_str!("app.js");
