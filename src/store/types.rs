//! Persistent entity types.

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    /// argon2id hash; never exposed over the API.
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Session,
    Bearer,
}

impl TokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenKind::Session => "session",
            TokenKind::Bearer => "bearer",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "bearer" => TokenKind::Bearer,
            _ => TokenKind::Session,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub user_id: i64,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "assistant" => MessageRole::Assistant,
            _ => MessageRole::User,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Sources attached to an assistant message (empty for user messages).
    pub sources: Vec<crate::engine::citations::Source>,
}
