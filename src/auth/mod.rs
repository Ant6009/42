//! Authentication: argon2id password hashing, token sessions, admin CLI.
//!
//! Design constraint (DESIGN.md): the token table carries a `kind` column
//! (session | bearer) and user creation lives in this service layer, so
//! bearer tokens and self-registration can be added later without rewrites.

use crate::config::Config;

pub mod service;

/// CLI: `42 admin create-user <name>` — prompts for a password, writes to SQLite.
pub fn cli_create_user(config: &Config, username: &str) -> anyhow::Result<()> {
    let _ = (config, username);
    todo!("prompt for password, hash with argon2id, insert user row")
}

/// CLI: `42 admin hash-password` — prompts, prints the argon2id hash.
pub fn cli_hash_password() -> anyhow::Result<()> {
    todo!("prompt for password, print argon2id hash")
}
