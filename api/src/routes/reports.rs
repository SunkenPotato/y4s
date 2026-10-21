use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::{check_write_limit, VerifiedUser};
use crate::db::is_unique_violation;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::ApiJson;
use crate::AppState;

pub const REASONS: &[&str] = &["spam", "offensive", "not_a_student", "wrong_info", "other"];

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ReportBody {
    target_type: String,
    target_id: Option<Uuid>,
    reason: String,
    details: Option<String>,
}

async fn open_report(state: &AppState, reporter: Uuid, target_type: &str, target_id: Uuid) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM reports WHERE reporter_id = $1 AND target_type = $2 AND target_id = $3 AND status = 'open'",
    )
    .bind(reporter)
    .bind(target_type)
    .bind(target_id)
    .fetch_optional(&state.db)
    .await?)
}

pub async fn create(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiJson(body): ApiJson<ReportBody>,
) -> AppResult<(StatusCode, Json<Value>)> {
    check_write_limit(&state, &user)?;
    let details = body.details.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
    let mut errors = FieldErrors::default();
    if !matches!(body.target_type.as_str(), "review" | "place") {
        errors.add("target_type", "Report a review or a place.");
    }
    if body.target_id.is_none() {
        errors.add("target_id", "Say which review or place you're reporting.");
    }
    if !REASONS.contains(&body.reason.as_str()) {
        errors.add("reason", "Pick a reason.");
    }
    if details.as_ref().is_some_and(|d| d.chars().count() > 1000) {
        errors.add("details", "Keep details under 1,000 characters.");
    }
    errors.check()?;
    let target_id = body.target_id.expect("validated");

    let exists: bool = match body.target_type.as_str() {
        "review" => sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM reviews WHERE id = $1 AND deleted_at IS NULL)"),
        _ => sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM places WHERE id = $1 AND status = 'active')"),
    }
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;
    if !exists {
        return Err(AppError::not_found());
    }

    // One open report per user per target; repeats return the existing id.
    if let Some(id) = open_report(&state, user.id, &body.target_type, target_id).await? {
        return Ok((StatusCode::OK, Json(json!({ "id": id }))));
    }
    let id = Uuid::new_v4();
    let res = sqlx::query(
        "INSERT INTO reports (id, reporter_id, target_type, target_id, reason, details) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user.id)
    .bind(&body.target_type)
    .bind(target_id)
    .bind(&body.reason)
    .bind(details)
    .execute(&state.db)
    .await;
    match res {
        Ok(_) => Ok((StatusCode::CREATED, Json(json!({ "id": id })))),
        Err(e) if is_unique_violation(&e) => {
            let id = open_report(&state, user.id, &body.target_type, target_id)
                .await?
                .ok_or_else(AppError::internal)?;
            Ok((StatusCode::OK, Json(json!({ "id": id }))))
        }
        Err(e) => Err(e.into()),
    }
}
