//! Authentication: argon2id password hashing, token sessions, admin CLI.
//!
//! Design constraint (DESIGN.md): the token table carries a `kind` column
//! (session | bearer) and user creation lives in this service layer, so
//! bearer tokens and self-registration can be added later without rewrites.

use crate::config::Config;
use crate::store::Store;

pub mod service;

pub use service::AuthService;

/// CLI: `42 admin create-user <name>` — prompts for a password (twice),
/// inserts the user as a regular user.
pub fn cli_create_user(
    config: &Config,
    username: &str,
    is_admin: bool,
) -> anyhow::Result<()> {
    let store = Store::open(std::path::Path::new(&config.database.path))?;
    let password = rpassword::prompt_password("Password: ")?;
    let confirm = rpassword::prompt_password("Confirm: ")?;
    if password != confirm {
        anyhow::bail!("passwords do not match");
    }
    let id = AuthService::create_user(&store, username, &password, is_admin)?;
    println!(
        "created user '{username}' (id {id}){}",
        if is_admin { " [admin]" } else { "" }
    );
    Ok(())
}

/// CLI: `42 admin hash-password` — prompts, prints the argon2id hash for
/// the admin seed in the TOML config.
pub fn cli_hash_password() -> anyhow::Result<()> {
    let password = rpassword::prompt_password("Password: ")?;
    println!("{}", AuthService::hash_password(&password)?);
    Ok(())
}
