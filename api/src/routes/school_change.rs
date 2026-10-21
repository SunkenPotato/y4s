//! Changing school: prove a new school email, then the account moves over atomically.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::db::is_unique_violation;
use crate::domains;
use crate::enrollment;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::ApiJson;
use crate::mail;
use crate::models::User;
use crate::queries;
use crate::routes::auth::{approved_school, email_exists, email_taken, not_school_email, VerifyBody};
use crate::verification::{self as v, SchoolChoice};
use crate::AppState;

#[derive(sqlx::FromRow)]
struct ChangeRow {
    email: String,
    email_domain: String,
    school_id: Option<Uuid>,
    enrolled_on: NaiveDate,
    code_hash: String,
    expires_at: DateTime<Utc>,
    attempts: i32,
    code_sent_at: DateTime<Utc>,
}

/// Verified and expired users can change school (both are stored as `verified`).
fn require_can_change(user: &User) -> AppResult<()> {
    if user.status != "verified" {
        return Err(AppError::forbidden("Finish verifying your current school email first."));
    }
    Ok(())
}

async fn lock_change(tx: &mut Transaction<'_, Postgres>, user_id: Uuid) -> AppResult<ChangeRow> {
    sqlx::query_as::<_, ChangeRow>("SELECT * FROM school_changes WHERE user_id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "not_found", "There's no school change in progress."))
}

async fn me_json(state: &AppState, user_id: Uuid) -> AppResult<Json<Value>> {
    let me = queries::me(&state.db, user_id).await?;
    Ok(Json(json!({ "user": me })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct RequestBody {
    email: String,
    enrolled_on: Option<String>,
}

pub async fn request(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<RequestBody>,
) -> AppResult<Json<Value>> {
    require_can_change(&user)?;
    let today = enrollment::today();
    let mut errors = FieldErrors::default();
    let parsed = domains::parse_email(&body.email);
    if parsed.is_none() {
        errors.add("email", "Enter your new school email, like name@nyu.edu.");
    }
    let enrolled_on = enrollment::parse_enrolled_on(&mut errors, body.enrolled_on.as_deref(), today);
    if parsed.as_ref().is_some_and(|(e, _)| *e == user.email) {
        errors.add("email", "Use your new school email. This one is already on your account.");
    }
    errors.check()?;
    let (email, domain) = parsed.expect("validated");
    let enrolled_on = enrolled_on.expect("validated");

    if domains::is_personal_provider(&domain) {
        return Err(not_school_email());
    }
    if enrollment::is_expired_on(enrolled_on, today) {
        return Err(enrollment::enrollment_too_old());
    }
    if email_exists(&state.db, &email).await? {
        return Err(email_taken());
    }

    let key = domains::domain_key(&domain);
    let school = approved_school(&state.db, &key).await?;
    let code = v::new_code();

    let mut tx = state.db.begin().await?;
    let last_sent: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT code_sent_at FROM school_changes WHERE user_id = $1 FOR UPDATE")
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await?;
    // Replacing a change sends a new email, so it shares the resend cooldown.
    if let Some(wait) = v::cooldown_left(last_sent) {
        return Err(AppError::rate_limited(wait));
    }
    sqlx::query(
        "INSERT INTO school_changes (user_id, email, email_domain, school_id, enrolled_on, code_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (user_id) DO UPDATE SET
           email = EXCLUDED.email, email_domain = EXCLUDED.email_domain, school_id = EXCLUDED.school_id,
           enrolled_on = EXCLUDED.enrolled_on, code_hash = EXCLUDED.code_hash, expires_at = EXCLUDED.expires_at,
           attempts = 0, code_sent_at = clock_timestamp(), created_at = now()",
    )
    .bind(user.id)
    .bind(&email)
    .bind(&key)
    .bind(school)
    .bind(enrolled_on)
    .bind(v::code_hash(user.id, &code))
    .bind(Utc::now() + Duration::minutes(v::CODE_TTL_MINUTES))
    .execute(&mut *tx)
    .await?;
    // Only keep the change if the code actually went out.
    state
        .mailer
        .send(mail::school_change_code_email(&email, &user.first_name, &code))
        .await?;
    tx.commit().await?;
    me_json(&state, user.id).await
}

pub async fn choose_school(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<SchoolChoice>,
) -> AppResult<Json<Value>> {
    require_can_change(&user)?;
    let chosen = v::resolve_school_choice(&state, body).await?;
    let mut tx = state.db.begin().await?;
    let change = lock_change(&mut tx, user.id).await?;
    let school_id = match change.school_id {
        None => v::link_domain(&mut tx, &change.email_domain, user.id, chosen).await?,
        Some(_) => v::repick_domain(&mut tx, &change.email_domain, user.id, chosen).await?,
    };
    sqlx::query("UPDATE school_changes SET school_id = $2 WHERE user_id = $1")
        .bind(user.id)
        .bind(school_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    me_json(&state, user.id).await
}

pub async fn verify(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<VerifyBody>,
) -> AppResult<Json<Value>> {
    require_can_change(&user)?;
    let code = v::parse_code(&body.code)?;
    let mut tx = state.db.begin().await?;
    let change = lock_change(&mut tx, user.id).await?;
    let Some(school_id) = change.school_id else {
        return Err(AppError::conflict(
            "school_choice_required",
            "Pick your new school before entering the code.",
        ));
    };
    if !v::check_code(user.id, code, &change.code_hash, change.expires_at, change.attempts)? {
        sqlx::query("UPDATE school_changes SET attempts = attempts + 1 WHERE user_id = $1")
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(v::invalid_code(change.attempts));
    }

    let dstatus = v::domain_status(&mut *tx, &change.email_domain).await?;
    let status = v::status_for_domain(dstatus.as_deref(), true);
    let role = v::role_after_verify(&state, &change.email, &user.role);
    // One statement swaps email, school and enrollment; reviews keep their own school snapshot.
    sqlx::query(
        "UPDATE users SET email = $2, email_domain = $3, school_id = $4, enrolled_on = $5, status = $6, role = $7,
                verified_at = now(), domain_decided_at = NULL
         WHERE id = $1",
    )
    .bind(user.id)
    .bind(&change.email)
    .bind(&change.email_domain)
    .bind(school_id)
    .bind(change.enrolled_on)
    .bind(status)
    .bind(role)
    .execute(&mut *tx)
    .await
    .map_err(|e| if is_unique_violation(&e) { email_taken() } else { e.into() })?;
    sqlx::query("DELETE FROM school_changes WHERE user_id = $1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    me_json(&state, user.id).await
}

pub async fn resend(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> AppResult<StatusCode> {
    require_can_change(&user)?;
    let mut tx = state.db.begin().await?;
    let change = lock_change(&mut tx, user.id).await?;
    if let Some(wait) = v::cooldown_left(Some(change.code_sent_at)) {
        return Err(AppError::rate_limited(wait));
    }
    let code = v::new_code();
    sqlx::query(
        "UPDATE school_changes SET code_hash = $2, expires_at = $3, attempts = 0, code_sent_at = clock_timestamp()
         WHERE user_id = $1",
    )
    .bind(user.id)
    .bind(v::code_hash(user.id, &code))
    .bind(Utc::now() + Duration::minutes(v::CODE_TTL_MINUTES))
    .execute(&mut *tx)
    .await?;
    state
        .mailer
        .send(mail::school_change_code_email(&change.email, &user.first_name, &code))
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> AppResult<StatusCode> {
    sqlx::query("DELETE FROM school_changes WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
