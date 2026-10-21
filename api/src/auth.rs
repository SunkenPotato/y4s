//! Passwords, session tokens, cookies, and the `CurrentUser` / `VerifiedUser` / `AdminUser` extractors.

use std::sync::OnceLock;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, HeaderValue};
use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::User;
use crate::AppState;

pub const SESSION_COOKIE: &str = "corner_session";
pub const SESSION_DAYS: i64 = 30;

pub fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| anyhow::anyhow!("hash task: {e}"))?
    .map_err(|e| AppError::from(anyhow::anyhow!("hash password: {e}")))
}

/// Hashes synchronously; used by the seed binary.
pub fn hash_password_blocking(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password")
        .to_string()
}

pub async fn verify_password(password: String, hash: Option<String>) -> bool {
    // Unknown emails still pay for one hash so response timing doesn't reveal which emails exist.
    static DUMMY: OnceLock<String> = OnceLock::new();
    tokio::task::spawn_blocking(move || {
        let hash = hash.unwrap_or_else(|| DUMMY.get_or_init(|| hash_password_blocking("corner-dummy")).clone());
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

pub fn new_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Creates a session row and returns the raw token for the cookie.
pub async fn create_session(db: &PgPool, user_id: Uuid) -> AppResult<String> {
    let token = new_session_token();
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(sha256_hex(&token))
        .bind(user_id)
        .bind(Utc::now() + Duration::days(SESSION_DAYS))
        .execute(db)
        .await?;
    Ok(token)
}

pub fn session_cookie(token: &str, secure: bool) -> HeaderValue {
    let mut v = format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        SESSION_DAYS * 24 * 3600
    );
    if secure {
        v.push_str("; Secure");
    }
    HeaderValue::from_str(&v).expect("cookie header")
}

pub fn clear_cookie(secure: bool) -> HeaderValue {
    let mut v = format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0");
    if secure {
        v.push_str("; Secure");
    }
    HeaderValue::from_str(&v).expect("cookie header")
}

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == SESSION_COOKIE && !value.is_empty())
        .map(|(_, value)| value.to_string())
}

pub async fn user_for_token(db: &PgPool, token: &str) -> AppResult<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(sha256_hex(token))
    .fetch_optional(db)
    .await?;
    Ok(user)
}

/// Any signed-in user, whatever their status.
pub struct CurrentUser(pub User);

/// A signed-in user with status `verified`.
pub struct VerifiedUser(pub User);

/// A signed-in user with role `admin`.
pub struct AdminUser(pub User);

/// The viewer if signed in; never rejects.
pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match session_token(&parts.headers) {
            Some(token) => Ok(MaybeUser(user_for_token(&state.db, &token).await?)),
            None => Ok(MaybeUser(None)),
        }
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let MaybeUser(user) = MaybeUser::from_request_parts(parts, state).await?;
        user.map(CurrentUser).ok_or_else(AppError::unauthenticated)
    }
}

impl FromRequestParts<AppState> for VerifiedUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if user.status != "verified" {
            return Err(AppError::not_verified());
        }
        Ok(VerifiedUser(user))
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if user.role != "admin" || user.status != "verified" {
            return Err(AppError::forbidden("Only admins can do this."));
        }
        Ok(AdminUser(user))
    }
}

/// Applies the per-user write limit (reviews, places, reports, votes).
pub fn check_write_limit(state: &AppState, user: &User) -> AppResult<()> {
    state
        .limiter
        .check(&format!("write:{}", user.id), state.config.write_rate_per_min)
}
