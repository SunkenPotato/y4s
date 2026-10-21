use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::{check_write_limit, CurrentUser, MaybeUser, VerifiedUser};
use crate::db::is_unique_violation;
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::models::{Review, ReviewRow, EDIT_WINDOW_MINUTES};
use crate::pagination::PageReq;
use crate::queries::{self, REVIEW_SELECT};
use crate::routes::places::visible_place;
use crate::AppState;

const RESTORE_WINDOW_MINUTES: i64 = 5;

#[derive(Deserialize)]
pub struct ReviewsQuery {
    sort: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    MaybeUser(viewer): MaybeUser,
    ApiPath(place_id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<ReviewsQuery>,
) -> AppResult<Json<Value>> {
    let page = PageReq::parse(q.limit, q.cursor.as_deref())?;
    let order = match q.sort.as_deref().unwrap_or("helpful") {
        "helpful" => "helpful_count DESC, r.created_at DESC, r.id",
        "new" => "r.created_at DESC, r.id",
        "lowest" => "r.rating ASC, r.created_at DESC, r.id",
        _ => return Err(AppError::field("sort", "Sort by helpful, new or lowest.")),
    };
    visible_place(&state, place_id, viewer.as_ref()).await?;
    let viewer_id = viewer.map(|u| u.id);
    let rows = sqlx::query_as::<_, ReviewRow>(&format!(
        "{REVIEW_SELECT} WHERE r.place_id = $2 AND r.deleted_at IS NULL ORDER BY {order} LIMIT $3 OFFSET $4"
    ))
    .bind(viewer_id)
    .bind(place_id)
    .bind(page.fetch_limit())
    .bind(page.offset)
    .fetch_all(&state.db)
    .await?;
    let items: Vec<Review> = rows.iter().map(|r| Review::from_row(r, viewer_id)).collect();
    Ok(Json(json!(page.finish(items))))
}

/// Distinguishes a missing key (`None`) from an explicit `null` (`Some(None)`).
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ReviewBody {
    rating: Option<i64>,
    body: Option<String>,
    #[serde(deserialize_with = "double_option")]
    visited_month: Option<Option<String>>,
    used_deal: Option<bool>,
}

fn validate_rating(errors: &mut FieldErrors, rating: Option<i64>) {
    if !rating.is_some_and(|r| (1..=5).contains(&r)) {
        errors.add("rating", "Pick a rating from 1 to 5 stars.");
    }
}

fn validate_body(errors: &mut FieldErrors, body: Option<&str>) {
    let n = body.map(|b| b.trim().chars().count()).unwrap_or(0);
    if n < 50 {
        errors.add("body", "Write at least 50 characters so it's useful.");
    } else if n > 2000 {
        errors.add("body", "Keep it under 2,000 characters.");
    }
}

/// "2026-10" -> first day of that month; rejects malformed or future months.
fn parse_month(errors: &mut FieldErrors, raw: &str) -> Option<NaiveDate> {
    let parsed = NaiveDate::parse_from_str(&format!("{}-01", raw.trim()), "%Y-%m-%d")
        .ok()
        .filter(|_| raw.trim().len() == 7);
    let today = Utc::now().date_naive();
    match parsed {
        None => {
            errors.add("visited_month", "Use the format YYYY-MM, like 2026-10.");
            None
        }
        Some(d) if (d.year(), d.month()) > (today.year(), today.month()) => {
            errors.add("visited_month", "Pick a month that isn't in the future.");
            None
        }
        Some(d) if d.year() < 2000 => {
            errors.add("visited_month", "Pick a month from 2000 onward.");
            None
        }
        Some(d) => Some(d),
    }
}

async fn load_review(state: &AppState, id: Uuid, viewer: Option<Uuid>) -> AppResult<ReviewRow> {
    queries::review_row(&state.db, id, viewer)
        .await?
        .ok_or_else(AppError::not_found)
}

async fn review_json(state: &AppState, id: Uuid, viewer: Uuid) -> AppResult<Value> {
    let row = load_review(state, id, Some(viewer)).await?;
    Ok(json!({ "review": Review::from_row(&row, Some(viewer)) }))
}

fn already_reviewed(review_id: Uuid) -> AppError {
    AppError::conflict(
        "already_reviewed",
        "You've already reviewed this place. Edit your review instead.",
    )
    .with("review_id", review_id.to_string())
}

async fn existing_review(state: &AppState, place_id: Uuid, user_id: Uuid) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM reviews WHERE place_id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(place_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?)
}

pub async fn create(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiPath(place_id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<ReviewBody>,
) -> AppResult<(StatusCode, Json<Value>)> {
    check_write_limit(&state, &user)?;
    let place = visible_place(&state, place_id, Some(&user)).await?;
    if place.status != "active" {
        return Err(AppError::forbidden("This place isn't open for reviews yet."));
    }
    let mut errors = FieldErrors::default();
    validate_rating(&mut errors, body.rating);
    validate_body(&mut errors, body.body.as_deref());
    let visited = match body.visited_month.flatten() {
        Some(m) => parse_month(&mut errors, &m),
        None => None,
    };
    errors.check()?;

    if let Some(existing) = existing_review(&state, place_id, user.id).await? {
        return Err(already_reviewed(existing));
    }
    let id = Uuid::new_v4();
    let inserted = sqlx::query(
        "INSERT INTO reviews (id, place_id, user_id, rating, body, visited_month, used_deal, school_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(place_id)
    .bind(user.id)
    .bind(body.rating.unwrap_or_default() as i16)
    .bind(body.body.unwrap_or_default().trim())
    .bind(visited)
    .bind(body.used_deal.unwrap_or(false))
    .bind(user.school_id)
    .execute(&state.db)
    .await;
    if let Err(e) = inserted {
        // Lost a race with a concurrent request from the same user.
        if is_unique_violation(&e) {
            if let Some(existing) = existing_review(&state, place_id, user.id).await? {
                return Err(already_reviewed(existing));
            }
        }
        return Err(e.into());
    }
    Ok((StatusCode::CREATED, Json(review_json(&state, id, user.id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<ReviewBody>,
) -> AppResult<Json<Value>> {
    check_write_limit(&state, &user)?;
    let review = load_review(&state, id, Some(user.id)).await?;
    if review.deleted_at.is_some() {
        return Err(AppError::not_found());
    }
    if review.user_id != user.id {
        return Err(AppError::forbidden("You can only edit your own reviews."));
    }
    if Utc::now() > review.created_at + Duration::minutes(EDIT_WINDOW_MINUTES) {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "edit_window_closed",
            "Reviews can be edited for 15 minutes after posting.",
        ));
    }
    let mut errors = FieldErrors::default();
    if body.rating.is_some() {
        validate_rating(&mut errors, body.rating);
    }
    if body.body.is_some() {
        validate_body(&mut errors, body.body.as_deref());
    }
    let visited = match &body.visited_month {
        None => review.visited_month,
        Some(None) => None,
        Some(Some(m)) => parse_month(&mut errors, m),
    };
    errors.check()?;

    let rating = body.rating.map(|r| r as i16).unwrap_or(review.rating);
    let text = body.body.map(|b| b.trim().to_string()).unwrap_or_else(|| review.body.clone());
    // Only a real change to what readers judge by (rating or text) marks the review as edited.
    let edited = rating != review.rating || text != review.body;
    sqlx::query(
        "UPDATE reviews SET rating = $2, body = $3, visited_month = $4, used_deal = $5, updated_at = now(),
                edited_at = CASE WHEN $6 THEN now() ELSE edited_at END
         WHERE id = $1",
    )
    .bind(id)
    .bind(rating)
    .bind(text)
    .bind(visited)
    .bind(body.used_deal.unwrap_or(review.used_deal))
    .bind(edited)
    .execute(&state.db)
    .await?;
    Ok(Json(review_json(&state, id, user.id).await?))
}

/// Deleted reviews are 404 except to their author during the restore window.
pub async fn get_review(
    State(state): State<AppState>,
    MaybeUser(viewer): MaybeUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    let viewer_id = viewer.as_ref().map(|u| u.id);
    let review = load_review(&state, id, viewer_id).await?;
    if let Some(deleted_at) = review.deleted_at {
        let restorable = viewer_id == Some(review.user_id)
            && Utc::now() - deleted_at <= Duration::minutes(RESTORE_WINDOW_MINUTES);
        if !restorable {
            return Err(AppError::not_found());
        }
    }
    visible_place(&state, review.place_id, viewer.as_ref()).await?;
    Ok(Json(json!({ "review": Review::from_row(&review, viewer_id) })))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<StatusCode> {
    let review = load_review(&state, id, Some(user.id)).await?;
    if review.deleted_at.is_some() {
        return Err(AppError::not_found());
    }
    if review.user_id != user.id && user.role != "admin" {
        return Err(AppError::forbidden("You can only delete your own reviews."));
    }
    sqlx::query("UPDATE reviews SET deleted_at = now(), deleted_by = $2 WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Undo for a delete: only the author, only for their own deletion, only within 5 minutes.
pub async fn restore(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    let review = load_review(&state, id, Some(user.id)).await?;
    if review.user_id != user.id {
        return Err(AppError::not_found());
    }
    let Some(deleted_at) = review.deleted_at else {
        return Ok(Json(review_json(&state, id, user.id).await?));
    };
    let in_window = Utc::now() - deleted_at <= Duration::minutes(RESTORE_WINDOW_MINUTES);
    if review.deleted_by != Some(user.id) || !in_window {
        return Err(AppError::forbidden("This review can no longer be restored."));
    }
    if let Some(existing) = existing_review(&state, review.place_id, user.id).await? {
        return Err(already_reviewed(existing));
    }
    sqlx::query("UPDATE reviews SET deleted_at = NULL, deleted_by = NULL WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| if is_unique_violation(&e) { already_reviewed(id) } else { e.into() })?;
    Ok(Json(review_json(&state, id, user.id).await?))
}

async fn vote_state(state: &AppState, review_id: Uuid, user_id: Uuid) -> AppResult<Json<Value>> {
    let (count, voted): (i64, bool) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM review_votes WHERE review_id = $1),
                EXISTS (SELECT 1 FROM review_votes WHERE review_id = $1 AND user_id = $2)",
    )
    .bind(review_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "helpful_count": count, "viewer_voted": voted })))
}

pub async fn vote(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    check_write_limit(&state, &user)?;
    let review = load_review(&state, id, Some(user.id)).await?;
    if review.deleted_at.is_some() {
        return Err(AppError::not_found());
    }
    if review.user_id == user.id {
        return Err(AppError::forbidden("You can't mark your own review as helpful."));
    }
    sqlx::query("INSERT INTO review_votes (review_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    vote_state(&state, id, user.id).await
}

pub async fn unvote(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    check_write_limit(&state, &user)?;
    let review = load_review(&state, id, Some(user.id)).await?;
    if review.deleted_at.is_some() {
        return Err(AppError::not_found());
    }
    sqlx::query("DELETE FROM review_votes WHERE review_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    vote_state(&state, id, user.id).await
}
