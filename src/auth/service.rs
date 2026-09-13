//! Auth service layer: user creation, password verification, session tokens.
//!
//! All user mutations go through here so that self-registration (later) is
//! just another caller, not a rewrite.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::store::types::{TokenKind, User};
use crate::store::Store;

pub struct AuthService;

impl AuthService {
    /// Hash a password with argon2id (random salt, default parameters).
    pub fn hash_password(password: &str) -> anyhow::Result<String> {
        let mut salt_bytes = [0u8; 16];
        use rand::RngCore as _;
        rand::thread_rng().fill_bytes(&mut salt_bytes);
        use base64::Engine as _;
        // password-hash salts are unpadded base64.
        let salt_b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(salt_bytes);
        let salt =
            SaltString::from_b64(&salt_b64).map_err(|e| anyhow::anyhow!("invalid salt: {e}"))?;
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("argon2 hashing failed: {e}"))?;
        Ok(hash.to_string())
    }

    pub fn verify_password(hash: &str, password: &str) -> anyhow::Result<bool> {
        let parsed =
            PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("invalid stored hash: {e}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    /// Create a user. Returns the new user id.
    pub fn create_user(
        store: &Store,
        username: &str,
        password: &str,
        is_admin: bool,
    ) -> anyhow::Result<i64> {
        validate_username(username)?;
        if let Some(existing) = store.get_user_by_username(username)? {
            anyhow::bail!("user '{}' already exists (id {})", username, existing.id);
        }
        let hash = Self::hash_password(password)?;
        store.create_user(username, &hash, is_admin)
    }

    /// Verify credentials; returns the user on success.
    pub fn authenticate(
        store: &Store,
        username: &str,
        password: &str,
    ) -> anyhow::Result<Option<User>> {
        let Some(user) = store.get_user_by_username(username)? else {
            // Constant-ish shape: run a dummy verify so timing does not
            // reveal whether the username exists.
            let dummy = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
            let _ = Self::verify_password(dummy, password);
            return Ok(None);
        };
        if Self::verify_password(&user.password_hash, password)? {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Create a session token for a user. Returns the raw token (shown once).
    pub fn create_session_token(
        store: &Store,
        user_id: i64,
        ttl: std::time::Duration,
    ) -> anyhow::Result<String> {
        let (_id, raw) = store.create_token(user_id, TokenKind::Session, Some(ttl))?;
        Ok(raw)
    }
}

fn validate_username(username: &str) -> anyhow::Result<()> {
    let len = username.len();
    if !(3..64).contains(&len) {
        anyhow::bail!("username must be 3-64 characters (got {len})");
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("username may only contain ASCII letters, digits, '-' and '_'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("auth.sqlite")).unwrap();
        (dir, store)
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = AuthService::hash_password("hunter2").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(AuthService::verify_password(&hash, "hunter2").unwrap());
        assert!(!AuthService::verify_password(&hash, "wrong").unwrap());
    }

    #[test]
    fn create_user_and_authenticate() {
        let (_dir, store) = temp_store();
        let id = AuthService::create_user(&store, "alice", "pw12345", false).unwrap();
        assert!(id > 0);

        // Duplicate rejected.
        assert!(AuthService::create_user(&store, "alice", "x", false).is_err());

        let user = AuthService::authenticate(&store, "alice", "pw12345")
            .unwrap()
            .unwrap();
        assert_eq!(user.username, "alice");
        assert!(!user.is_admin);

        assert!(AuthService::authenticate(&store, "alice", "nope")
            .unwrap()
            .is_none());
        assert!(AuthService::authenticate(&store, "ghost", "pw12345")
            .unwrap()
            .is_none());
    }

    #[test]
    fn username_validation() {
        let (_dir, store) = temp_store();
        assert!(AuthService::create_user(&store, "ab", "pw12345", false).is_err());
        assert!(AuthService::create_user(&store, "bad name", "pw12345", false).is_err());
        assert!(AuthService::create_user(&store, "ok-name_1", "pw12345", false).is_ok());
    }

    #[test]
    fn session_token_works() {
        let (_dir, store) = temp_store();
        let id = AuthService::create_user(&store, "alice", "pw12345", false).unwrap();
        let raw =
            AuthService::create_session_token(&store, id, std::time::Duration::from_secs(3600))
                .unwrap();
        assert_eq!(raw.len(), 64);
        let (kind, user) = store.validate_token(&raw).unwrap().unwrap();
        assert_eq!(kind, TokenKind::Session);
        assert_eq!(user.id, id);
    }
}
