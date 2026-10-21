//! Shared pieces of the student-verification flows (signup and school change): codes, school
//! choice, and linking an email domain to a school.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::sha256_hex;
use crate::domains;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::models::User;
use crate::queries;
use crate::AppState;

pub const CODE_TTL_MINUTES: i64 = 15;
pub const CODE_MAX_ATTEMPTS: i32 = 5;
pub const RESEND_COOLDOWN_SECS: i64 = 60;

pub fn new_code() -> String {
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}

/// Codes are salted with the user id before hashing.
pub fn code_hash(user_id: Uuid, code: &str) -> String {
    sha256_hex(&format!("{user_id}:{code}"))
}

pub fn parse_code(raw: &str) -> AppResult<&str> {
    let code = raw.trim();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::field("code", "Enter the 6-digit code from your email."));
    }
    Ok(code)
}

/// Checks a stored code. `Ok(true)` on a match, `Ok(false)` on a wrong code (the caller records
/// the attempt and answers with `invalid_code(...)`), or the lockout/expiry errors.
pub fn check_code(
    user_id: Uuid,
    code: &str,
    hash: &str,
    expires_at: DateTime<Utc>,
    attempts: i32,
) -> AppResult<bool> {
    if attempts >= CODE_MAX_ATTEMPTS {
        return Err(AppError::unprocessable(
            "too_many_attempts",
            "Too many wrong codes. Request a new code and try again.",
        ));
    }
    if expires_at <= Utc::now() {
        return Err(code_expired());
    }
    Ok(subtle::ConstantTimeEq::ct_eq(code_hash(user_id, code).as_bytes(), hash.as_bytes()).into())
}

/// `attempts` is the count before this wrong try.
pub fn invalid_code(attempts: i32) -> AppError {
    AppError::unprocessable(
        "invalid_code",
        "That code isn't right. Check the latest email from Corner and try again.",
    )
    .with("attempts_left", CODE_MAX_ATTEMPTS - attempts - 1)
}

pub fn code_expired() -> AppError {
    AppError::unprocessable("code_expired", "This code has expired. Request a new code.")
}

/// Seconds left on the resend cooldown, if any.
pub fn cooldown_left(last_sent: Option<DateTime<Utc>>) -> Option<u64> {
    let elapsed = (Utc::now() - last_sent?).num_seconds();
    (elapsed < RESEND_COOLDOWN_SECS).then(|| (RESEND_COOLDOWN_SECS - elapsed).max(1) as u64)
}

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

pub async fn domain_status(db: impl sqlx::PgExecutor<'_>, domain: &str) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT status FROM school_domains WHERE domain = $1")
            .bind(domain)
            .fetch_optional(db)
            .await?,
    )
}

/// `ADMIN_EMAILS` grants the admin role once the email is confirmed.
pub fn role_after_verify(state: &AppState, email: &str, current: &str) -> String {
    if state.config.is_admin_email(email) {
        "admin".into()
    } else {
        current.to_string()
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct NewSchool {
    name: String,
    kind: String,
    city: String,
    state: String,
}

/// Body of `/api/auth/school` and `/api/me/school-change/school`.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct SchoolChoice {
    school_id: Option<Uuid>,
    new_school: Option<NewSchool>,
}

/// Resolves the body to an existing school id, creating the school for `new_school`.
pub async fn resolve_school_choice(state: &AppState, body: SchoolChoice) -> AppResult<Uuid> {
    match (body.school_id, body.new_school) {
        (Some(id), None) => {
            queries::school(&state.db, id).await?.ok_or_else(AppError::not_found)?;
            Ok(id)
        }
        (None, Some(ns)) => find_or_create_school(state, ns).await,
        _ => Err(AppError::field("school_id", "Pick your school or add a new one.")),
    }
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

/// First pick: links `domain` to `chosen` (approved for .edu, pending otherwise) unless a row
/// already exists, in which case the existing row's school wins. Returns the user's school.
pub async fn link_domain(
    tx: &mut Transaction<'_, Postgres>,
    domain: &str,
    requester: Uuid,
    chosen: Uuid,
) -> AppResult<Uuid> {
    let approved = domains::is_edu(domain);
    sqlx::query(
        "INSERT INTO school_domains (domain, school_id, status, requested_by, decided_at)
         VALUES ($1, $2, $3, $4, CASE WHEN $3 = 'approved' THEN now() END)
         ON CONFLICT (domain) DO NOTHING",
    )
    .bind(domain)
    .bind(chosen)
    .bind(if approved { "approved" } else { "pending" })
    .bind(requester)
    .execute(&mut **tx)
    .await?;
    let school_id: Uuid = sqlx::query_scalar("SELECT school_id FROM school_domains WHERE domain = $1")
        .bind(domain)
        .fetch_one(&mut **tx)
        .await?;
    Ok(school_id)
}

/// Re-pick ("Picked the wrong school?"). An approved domain can't move. A pending or rejected
/// request moves to `chosen` and goes back to pending, unless other students are waiting on it.
pub async fn repick_domain(
    tx: &mut Transaction<'_, Postgres>,
    domain: &str,
    requester: Uuid,
    chosen: Uuid,
) -> AppResult<Uuid> {
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT d.school_id, d.status, s.name FROM school_domains d JOIN schools s ON s.id = d.school_id
         WHERE d.domain = $1 FOR UPDATE OF d",
    )
    .bind(domain)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((current, status, current_name)) = row else {
        return link_domain(tx, domain, requester, chosen).await;
    };
    if status == "approved" {
        return Err(AppError::forbidden(
            "Your email domain is already confirmed for this school, so it can't be changed.",
        ));
    }
    if current == chosen {
        return Ok(current);
    }
    // Others who already picked through this domain: signups that set a school, plus school changes.
    let others: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM users WHERE email_domain = $1 AND id <> $2 AND school_id IS NOT NULL
                   AND status IN ('pending_email', 'pending_school', 'rejected'))
              + (SELECT count(*) FROM school_changes WHERE email_domain = $1 AND user_id <> $2
                   AND school_id IS NOT NULL)",
    )
    .bind(domain)
    .bind(requester)
    .fetch_one(&mut **tx)
    .await?;
    if others > 0 {
        return Err(AppError::conflict(
            "school_locked",
            format!("Other students from this email domain already picked {current_name}. Contact us if that's wrong."),
        ));
    }
    sqlx::query(
        "UPDATE school_domains SET school_id = $2, status = 'pending', requested_by = $3, requested_at = now(),
                decided_by = NULL, decided_at = NULL, reject_reason = NULL
         WHERE domain = $1",
    )
    .bind(domain)
    .bind(chosen)
    .bind(requester)
    .execute(&mut **tx)
    .await?;
    Ok(chosen)
}

/// Statuses that may pick (or re-pick) a school during signup.
pub fn can_pick_school(user: &User) -> bool {
    matches!(user.status.as_str(), "pending_email" | "pending_school" | "rejected")
}
