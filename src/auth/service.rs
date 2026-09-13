//! Auth service layer: user creation, password verification, session tokens.
//!
//! All user mutations go through here so that self-registration (later) is
//! just another caller, not a rewrite.

pub struct AuthService;
