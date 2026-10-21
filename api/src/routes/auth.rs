use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{Datelike, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{self, sha256_hex, CurrentUser};
use crate::domains;
use crate::enrollment;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::ApiJson;
use crate::mail;
use crate::models::User;
use crate::queries;
use crate::verification::{self as v, SchoolChoice};
use crate::AppState;

/// Creates a fresh code (invalidating older ones) and emails it. With `strict`, a failed send rolls
/// the new code back and returns `mail_failed`; otherwise the failure is only logged.
async fn issue_code(state: &AppState, user: &User, strict: bool) -> AppResult<()> {
    let code = v::new_code();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM verification_codes WHERE user_id = $1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO verification_codes (id, user_id, code_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(v::code_hash(user.id, &code))
    .bind(Utc::now() + Duration::minutes(v::CODE_TTL_MINUTES))
    .execute(&mut *tx)
    .await?;
    let email = mail::code_email(&user.email, &user.first_name, &code);
    if strict {
        state.mailer.send(email).await?;
        tx.commit().await?;
    } else {
        tx.commit().await?;
        mail::send_logged(&state.mailer, email).await;
    }
    Ok(())
}

fn validate_name(errors: &mut FieldErrors, field: &str, value: &str) {
    let n = value.trim().chars().count();
    if n == 0 {
        errors.add(field, "Enter your name.");
    } else if n > 40 {
        errors.add(field, "Use 40 characters or fewer.");
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct SignupBody {
    email: String,
    password: String,
    first_name: String,
    last_name: String,
    birth_year: Option<i32>,
    enrolled_on: Option<String>,
}

pub async fn signup(State(state): State<AppState>, ApiJson(body): ApiJson<SignupBody>) -> AppResult<impl IntoResponse> {
    let today = enrollment::today();
    let mut errors = FieldErrors::default();
    let parsed = domains::parse_email(&body.email);
    if parsed.is_none() {
        errors.add("email", "Enter a valid email, like name@ucla.edu.");
    }
    let pw_len = body.password.chars().count();
    if pw_len < 10 {
        errors.add("password", "Use at least 10 characters.");
    } else if pw_len > 200 {
        errors.add("password", "Use 200 characters or fewer.");
    }
    validate_name(&mut errors, "first_name", &body.first_name);
    validate_name(&mut errors, "last_name", &body.last_name);
    let this_year = Utc::now().year();
    match body.birth_year {
        None => errors.add("birth_year", "Enter the year you were born."),
        Some(y) if y > this_year || y < this_year - 120 => {
            errors.add("birth_year", "Enter a real birth year, like 2005.")
        }
        _ => {}
    }
    let enrolled_on = enrollment::parse_enrolled_on(&mut errors, body.enrolled_on.as_deref(), today);
    errors.check()?;
    let (email, domain) = parsed.expect("validated");
    let birth_year = body.birth_year.expect("validated");
    let enrolled_on = enrolled_on.expect("validated");

    if domains::is_personal_provider(&domain) {
        return Err(not_school_email());
    }
    if this_year - birth_year < 13 {
        return Err(AppError::unprocessable(
            "too_young",
            "You need to be at least 13 to join Corner.",
        ));
    }
    if enrollment::is_expired_on(enrolled_on, today) {
        return Err(enrollment::enrollment_too_old());
    }
    if email_exists(&state.db, &email).await? {
        return Err(email_taken());
    }

    let key = domains::domain_key(&domain);
    let approved_school = approved_school(&state.db, &key).await?;

    let password_hash = auth::hash_password(body.password).await?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, email_domain, password_hash, first_name, last_name, birth_year, school_id,
                            enrolled_on)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&key)
    .bind(password_hash)
    .bind(body.first_name.trim())
    .bind(body.last_name.trim())
    .bind(birth_year)
    .bind(approved_school)
    .bind(enrolled_on)
    .fetch_one(&state.db)
    .await
    .map_err(|e| if crate::db::is_unique_violation(&e) { email_taken() } else { e.into() })?;

    // The account exists either way; if the email fails the user can ask for a resend.
    issue_code(&state, &user, false).await?;
    let token = auth::create_session(&state.db, user.id).await?;
    let me = queries::me(&state.db, user.id).await?;
    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, auth::session_cookie(&token, state.config.cookie_secure))],
        Json(json!({ "user": me })),
    ))
}

pub fn not_school_email() -> AppError {
    AppError::unprocessable(
        "not_school_email",
        "Use the email your school gave you, like name@ucla.edu.",
    )
}

pub fn email_taken() -> AppError {
    AppError::conflict("email_taken", "An account with this email already exists. Log in instead.")
}

pub async fn email_exists(db: &PgPool, email: &str) -> AppResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(email)
        .fetch_one(db)
        .await?)
}

pub async fn approved_school(db: &PgPool, domain_key: &str) -> AppResult<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT school_id FROM school_domains WHERE domain = $1 AND status = 'approved'")
            .bind(domain_key)
            .fetch_optional(db)
            .await?,
    )
}

pub async fn choose_school(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<SchoolChoice>,
) -> AppResult<Json<serde_json::Value>> {
    if !v::can_pick_school(&user) {
        return Err(AppError::forbidden("Your school is already set."));
    }
    let chosen = v::resolve_school_choice(&state, body).await?;

    let mut tx = state.db.begin().await?;
    let school_id = match user.school_id {
        None => v::link_domain(&mut tx, &user.email_domain, user.id, chosen).await?,
        Some(_) => v::repick_domain(&mut tx, &user.email_domain, user.id, chosen).await?,
    };
    let domain_status = v::domain_status(&mut *tx, &user.email_domain).await?;
    // Users who already confirmed their email get their final status right away.
    let (status, role) = if user.verified_at.is_some() {
        let status = v::status_for_domain(domain_status.as_deref(), true);
        (status.to_string(), v::role_after_verify(&state, &user.email, &user.role))
    } else {
        (user.status.clone(), user.role.clone())
    };
    sqlx::query(
        "UPDATE users SET school_id = $2, status = $3, role = $4, domain_decided_at = NULL WHERE id = $1",
    )
    .bind(user.id)
    .bind(school_id)
    .bind(&status)
    .bind(&role)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let me = queries::me(&state.db, user.id).await?;
    Ok(Json(json!({ "user": me })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct VerifyBody {
    pub code: String,
}

pub async fn verify(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<VerifyBody>,
) -> AppResult<Json<serde_json::Value>> {
    if user.verified_at.is_some() {
        return Err(already_verified());
    }
    let code = v::parse_code(&body.code)?;

    let mut tx = state.db.begin().await?;
    let row: Option<(Uuid, String, chrono::DateTime<Utc>, i32)> = sqlx::query_as(
        "SELECT id, code_hash, expires_at, attempts FROM verification_codes
         WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((code_id, hash, expires_at, attempts)) = row else {
        return Err(v::code_expired());
    };
    if !v::check_code(user.id, code, &hash, expires_at, attempts)? {
        sqlx::query("UPDATE verification_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(v::invalid_code(attempts));
    }

    let dstatus = v::domain_status(&mut *tx, &user.email_domain).await?;
    let status = v::status_for_domain(dstatus.as_deref(), user.school_id.is_some());
    let role = v::role_after_verify(&state, &user.email, &user.role);
    sqlx::query("DELETE FROM verification_codes WHERE user_id = $1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET verified_at = now(), status = $2, role = $3 WHERE id = $1")
        .bind(user.id)
        .bind(status)
        .bind(role)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let me = queries::me(&state.db, user.id).await?;
    Ok(Json(json!({ "user": me })))
}

fn already_verified() -> AppError {
    AppError::conflict("already_verified", "Your email is already confirmed.")
}

pub async fn resend(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> AppResult<StatusCode> {
    if user.verified_at.is_some() {
        return Err(already_verified());
    }
    let last: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT max(created_at) FROM verification_codes WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    if let Some(wait) = v::cooldown_left(last) {
        return Err(AppError::rate_limited(wait));
    }
    issue_code(&state, &user, true).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LoginBody {
    email: String,
    password: String,
}

pub async fn login(State(state): State<AppState>, ApiJson(body): ApiJson<LoginBody>) -> AppResult<impl IntoResponse> {
    let email = body.email.trim().to_lowercase();
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await?;
    let ok = auth::verify_password(body.password, user.as_ref().map(|u| u.password_hash.clone())).await;
    let user = match user {
        Some(u) if ok => u,
        _ => {
            return Err(AppError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "That email and password don't match. Check them and try again.",
            ))
        }
    };
    if user.status == "suspended" {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "suspended",
            "This account is suspended. Contact Corner support if you think that's a mistake.",
        ));
    }
    let token = auth::create_session(&state.db, user.id).await?;
    let me = queries::me(&state.db, user.id).await?;
    Ok((
        [(header::SET_COOKIE, auth::session_cookie(&token, state.config.cookie_secure))],
        Json(json!({ "user": me })),
    ))
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<impl IntoResponse> {
    if let Some(token) = auth::session_token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(sha256_hex(&token))
            .execute(&state.db)
            .await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, auth::clear_cookie(state.config.cookie_secure))],
    ))
}
