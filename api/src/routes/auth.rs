use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{Datelike, Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::{self, sha256_hex, CurrentUser};
use crate::domains;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::ApiJson;
use crate::mail;
use crate::models::User;
use crate::queries;
use crate::AppState;

pub const CODE_TTL_MINUTES: i64 = 15;
pub const CODE_MAX_ATTEMPTS: i32 = 5;
pub const RESEND_COOLDOWN_SECS: i64 = 60;

/// The user status once their email is confirmed, given the state of their domain.
pub fn status_for_domain(domain_status: Option<&str>, has_school: bool) -> &'static str {
    if !has_school {
        return "pending_school";
    }
    match domain_status {
        Some("approved") => "verified",
        Some("rejected") => "rejected",
        _ => "pending_school",
    }
}

async fn domain_status(db: impl sqlx::PgExecutor<'_>, domain: &str) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT status FROM school_domains WHERE domain = $1")
            .bind(domain)
            .fetch_optional(db)
            .await?,
    )
}

fn code_hash(user_id: Uuid, code: &str) -> String {
    sha256_hex(&format!("{user_id}:{code}"))
}

/// Creates a fresh 6-digit code (invalidating older ones) and emails it.
async fn issue_code(state: &AppState, user: &User) -> AppResult<()> {
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
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
    .bind(code_hash(user.id, &code))
    .bind(Utc::now() + Duration::minutes(CODE_TTL_MINUTES))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    mail::send_logged(&state.mailer, mail::code_email(&user.email, &user.first_name, &code)).await;
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
}

pub async fn signup(State(state): State<AppState>, ApiJson(body): ApiJson<SignupBody>) -> AppResult<impl IntoResponse> {
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
    errors.check()?;
    let (email, domain) = parsed.expect("validated");
    let birth_year = body.birth_year.expect("validated");

    if domains::is_personal_provider(&domain) {
        return Err(AppError::unprocessable(
            "not_school_email",
            "Use the email your school gave you, like name@ucla.edu.",
        ));
    }
    if this_year - birth_year < 13 {
        return Err(AppError::unprocessable(
            "too_young",
            "You need to be at least 13 to join Corner.",
        ));
    }

    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(&state.db)
        .await?;
    if taken {
        return Err(email_taken());
    }

    let key = domains::domain_key(&domain);
    let approved_school: Option<Uuid> =
        sqlx::query_scalar("SELECT school_id FROM school_domains WHERE domain = $1 AND status = 'approved'")
            .bind(&key)
            .fetch_optional(&state.db)
            .await?;

    let password_hash = auth::hash_password(body.password).await?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, email_domain, password_hash, first_name, last_name, birth_year, school_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&key)
    .bind(password_hash)
    .bind(body.first_name.trim())
    .bind(body.last_name.trim())
    .bind(birth_year)
    .bind(approved_school)
    .fetch_one(&state.db)
    .await
    .map_err(|e| if crate::db::is_unique_violation(&e) { email_taken() } else { e.into() })?;

    issue_code(&state, &user).await?;
    let token = auth::create_session(&state.db, user.id).await?;
    let me = queries::me(&state.db, user.id).await?;
    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, auth::session_cookie(&token, state.config.cookie_secure))],
        Json(json!({ "user": me })),
    ))
}

fn email_taken() -> AppError {
    AppError::conflict("email_taken", "An account with this email already exists. Log in instead.")
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct NewSchool {
    name: String,
    kind: String,
    city: String,
    state: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChooseSchoolBody {
    school_id: Option<Uuid>,
    new_school: Option<NewSchool>,
}

pub async fn choose_school(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<ChooseSchoolBody>,
) -> AppResult<Json<serde_json::Value>> {
    if user.school_id.is_some() {
        return Err(AppError::forbidden("Your school is already set."));
    }
    let chosen = match (body.school_id, body.new_school) {
        (Some(id), None) => {
            queries::school(&state.db, id).await?.ok_or_else(AppError::not_found)?;
            id
        }
        (None, Some(ns)) => find_or_create_school(&state, ns).await?,
        _ => return Err(AppError::field("school_id", "Pick your school or add a new one.")),
    };

    let mut tx = state.db.begin().await?;
    let school_id = link_domain(&mut tx, &user, chosen).await?;
    let domain_status = domain_status(&mut *tx, &user.email_domain).await?;
    // Users who already confirmed their email get their final status right away.
    let (status, role) = if user.verified_at.is_some() {
        let status = status_for_domain(domain_status.as_deref(), true);
        (status.to_string(), role_after_verify(&state, &user))
    } else {
        (user.status.clone(), user.role.clone())
    };
    sqlx::query("UPDATE users SET school_id = $2, status = $3, role = $4 WHERE id = $1")
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

/// `ADMIN_EMAILS` grants the admin role once the email is confirmed.
fn role_after_verify(state: &AppState, user: &User) -> String {
    if state.config.is_admin_email(&user.email) {
        "admin".into()
    } else {
        user.role.clone()
    }
}

/// Links the user's domain to `chosen` (approved for .edu, pending otherwise) unless a row already
/// exists, in which case the existing row's school wins. Returns the school the user belongs to.
async fn link_domain(tx: &mut Transaction<'_, Postgres>, user: &User, chosen: Uuid) -> AppResult<Uuid> {
    let domain = &user.email_domain;
    let approved = domains::is_edu(domain);
    sqlx::query(
        "INSERT INTO school_domains (domain, school_id, status, requested_by, decided_at)
         VALUES ($1, $2, $3, $4, CASE WHEN $3 = 'approved' THEN now() END)
         ON CONFLICT (domain) DO NOTHING",
    )
    .bind(domain)
    .bind(chosen)
    .bind(if approved { "approved" } else { "pending" })
    .bind(user.id)
    .execute(&mut **tx)
    .await?;
    let school_id: Uuid = sqlx::query_scalar("SELECT school_id FROM school_domains WHERE domain = $1")
        .bind(domain)
        .fetch_one(&mut **tx)
        .await?;
    Ok(school_id)
}

async fn find_or_create_school(state: &AppState, ns: NewSchool) -> AppResult<Uuid> {
    let name = ns.name.trim().to_string();
    let city = ns.city.trim().to_string();
    let us_state = ns.state.trim().to_uppercase();
    let mut errors = FieldErrors::default();
    let n = name.chars().count();
    if !(2..=120).contains(&n) {
        errors.add("new_school.name", "Enter your school's full name.");
    }
    if !matches!(ns.kind.as_str(), "university" | "high_school") {
        errors.add("new_school.kind", "Pick university or high school.");
    }
    if city.is_empty() || city.chars().count() > 80 {
        errors.add("new_school.city", "Enter the city your school is in.");
    }
    if us_state.len() != 2 || !us_state.chars().all(|c| c.is_ascii_alphabetic()) {
        errors.add("new_school.state", "Use the two-letter state code, like CA.");
    }
    errors.check()?;

    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM schools WHERE lower(name) = lower($1) AND lower(city) = lower($2) AND state = $3 LIMIT 1",
    )
    .bind(&name)
    .bind(&city)
    .bind(&us_state)
    .fetch_optional(&state.db)
    .await?;
    if let Some(id) = existing {
        return Ok(id);
    }
    // Coordinates are nice to have for the feed; a geocoder miss shouldn't block signup.
    let coords = state
        .geocoder
        .geocode(&format!("{name}, {city}, {us_state}"))
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "school geocode failed");
            None
        });
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO schools (id, name, kind, city, state, lat, lng) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(id)
        .bind(&name)
        .bind(&ns.kind)
        .bind(&city)
        .bind(&us_state)
        .bind(coords.map(|c| c.0))
        .bind(coords.map(|c| c.1))
        .execute(&state.db)
        .await?;
    Ok(id)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct VerifyBody {
    code: String,
}

pub async fn verify(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<VerifyBody>,
) -> AppResult<Json<serde_json::Value>> {
    let code = body.code.trim();
    if user.verified_at.is_some() {
        return Err(already_verified());
    }
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::field("code", "Enter the 6-digit code from your email."));
    }

    let mut tx = state.db.begin().await?;
    let row: Option<(Uuid, String, chrono::DateTime<Utc>, i32)> = sqlx::query_as(
        "SELECT id, code_hash, expires_at, attempts FROM verification_codes
         WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((code_id, hash, expires_at, attempts)) = row else {
        return Err(code_expired());
    };
    if attempts >= CODE_MAX_ATTEMPTS {
        return Err(AppError::unprocessable(
            "too_many_attempts",
            "Too many wrong codes. Request a new code and try again.",
        ));
    }
    if expires_at <= Utc::now() {
        return Err(code_expired());
    }
    let matches: bool = subtle::ConstantTimeEq::ct_eq(code_hash(user.id, code).as_bytes(), hash.as_bytes()).into();
    if !matches {
        sqlx::query("UPDATE verification_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let left = CODE_MAX_ATTEMPTS - attempts - 1;
        return Err(AppError::unprocessable(
            "invalid_code",
            "That code isn't right. Check the latest email from Corner and try again.",
        )
        .with("attempts_left", left));
    }

    let dstatus = domain_status(&mut *tx, &user.email_domain).await?;
    let status = status_for_domain(dstatus.as_deref(), user.school_id.is_some());
    let role = role_after_verify(&state, &user);
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

fn code_expired() -> AppError {
    AppError::unprocessable("code_expired", "This code has expired. Request a new code.")
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
    if let Some(last) = last {
        let elapsed = (Utc::now() - last).num_seconds();
        if elapsed < RESEND_COOLDOWN_SECS {
            return Err(AppError::rate_limited((RESEND_COOLDOWN_SECS - elapsed).max(1) as u64));
        }
    }
    issue_code(&state, &user).await?;
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
        delete_session(&state.db, &token).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, auth::clear_cookie(state.config.cookie_secure))],
    ))
}

async fn delete_session(db: &PgPool, token: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(sha256_hex(token))
        .execute(db)
        .await?;
    Ok(())
}
