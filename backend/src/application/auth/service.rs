use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use rand::{Rng, distributions::Alphanumeric};

use uuid::Uuid;

use crate::domain::auth::{
    entities::{Session, User, UserStatus},
    repository::{SessionRepository, UserRepository},
};
use crate::errors::{AppError, AppResult};

/// Session lifetime. Kept simple: a fixed TTL, no sliding refresh.
const SESSION_TTL_HOURS: i64 = 24 * 7;

pub struct AuthService {
    pub users: Arc<dyn UserRepository>,
    pub sessions: Arc<dyn SessionRepository>,
}

impl AuthService {
    pub fn new(users: Arc<dyn UserRepository>, sessions: Arc<dyn SessionRepository>) -> Self {
        Self { users, sessions }
    }

    /// Verify credentials and open a new session. Returns the user and the
    /// opaque session token to hand back as a cookie.
    pub async fn login(&self, email: &str, password: &str) -> AppResult<(User, String)> {
        let user = self
            .users
            .find_by_email(&email.trim().to_lowercase())
            .await?;

        // Always run a verification (against a dummy hash when the user is
        // unknown) so timing does not reveal whether the email exists.
        let user = match user {
            Some(u) => {
                if verify_password(password, &u.password_hash) {
                    u
                } else {
                    return Err(AppError::Unauthorized);
                }
            }
            None => {
                let _ = verify_password(password, dummy_hash());
                return Err(AppError::Unauthorized);
            }
        };

        let now = Utc::now();
        let session = Session {
            token: random_token(48),
            user_id: user.id,
            created_at: now,
            expires_at: now + Duration::hours(SESSION_TTL_HOURS),
        };
        let token = session.token.clone();
        self.sessions.create(session).await?;

        Ok((user, token))
    }

    /// Resolve the user behind a session token, or `Unauthorized`.
    pub async fn authenticate(&self, token: &str) -> AppResult<User> {
        let session = self
            .sessions
            .find(token)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if session.is_expired(Utc::now()) {
            let _ = self.sessions.delete(token).await;
            return Err(AppError::Unauthorized);
        }

        let user = self
            .users
            .find_by_id(session.user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        // A user who has since been suspended/deactivated must not keep a live
        // session — drop it so every other session of theirs dies too.
        if user.status != UserStatus::Active {
            let _ = self.sessions.delete_for_user(user.id).await;
            return Err(AppError::Unauthorized);
        }

        Ok(user)
    }

    pub async fn logout(&self, token: &str) -> AppResult<()> {
        self.sessions.delete(token).await
    }

    /// Invalidate every session for a user (password change, suspend, delete).
    pub async fn invalidate_user_sessions(&self, user_id: Uuid) -> AppResult<()> {
        self.sessions.delete_for_user(user_id).await
    }
}

/// A real Argon2id hash computed once, used to keep login timing roughly
/// constant when the account does not exist.
fn dummy_hash() -> &'static str {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password("timing-equaliser-not-a-real-password")
            .unwrap_or_else(|_| String::from("$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))
    })
}

/// Hash a plaintext secret (password, API token) with Argon2id.
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("password hash failed: {e}")))
}

/// Constant-time-ish verification of a plaintext secret against a stored PHC hash.
pub fn verify_password(password: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A URL-safe random token of `len` alphanumeric characters.
pub fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
