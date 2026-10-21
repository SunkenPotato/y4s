//! Saved places. Any signed-in user can save, whatever their status.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::extract::{ApiPath, ApiQuery};
use crate::pagination::PageReq;
use crate::queries::{self, PlaceFilter, PlaceSort};
use crate::routes::places::visible_place;
use crate::AppState;

pub async fn save(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    visible_place(&state, id, Some(&user)).await?;
    sqlx::query("INSERT INTO saved_places (user_id, place_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user.id)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "saved": true })))
}

pub async fn unsave(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM places WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    if !exists {
        return Err(AppError::not_found());
    }
    sqlx::query("DELETE FROM saved_places WHERE user_id = $1 AND place_id = $2")
        .bind(user.id)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "saved": false })))
}

#[derive(Deserialize)]
pub struct SavedQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiQuery(q): ApiQuery<SavedQuery>,
) -> AppResult<Json<Value>> {
    let page = PageReq::parse(q.limit, q.cursor.as_deref())?;
    let rows = queries::fetch_place_rows(
        &state.db,
        &PlaceFilter {
            saved_by: Some(user.id),
            sort: PlaceSort::Saved,
            limit: page.fetch_limit(),
            offset: page.offset,
            ..Default::default()
        },
    )
    .await?;
    let places = queries::places_from_rows(&state.db, &rows).await?;
    Ok(Json(json!(page.finish(places))))
}
