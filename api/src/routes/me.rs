use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{AppResult, FieldErrors};
use crate::extract::ApiJson;
use crate::queries;
use crate::AppState;

pub async fn get_me(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> AppResult<Json<Value>> {
    let me = queries::me(&state.db, user.id).await?;
    Ok(Json(json!({ "user": me })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PatchMe {
    first_name: Option<String>,
    last_name: Option<String>,
}

pub async fn patch_me(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(body): ApiJson<PatchMe>,
) -> AppResult<Json<Value>> {
    let mut errors = FieldErrors::default();
    for (field, value) in [("first_name", &body.first_name), ("last_name", &body.last_name)] {
        if let Some(v) = value {
            let n = v.trim().chars().count();
            if n == 0 {
                errors.add(field, "Enter your name.");
            } else if n > 40 {
                errors.add(field, "Use 40 characters or fewer.");
            }
        }
    }
    errors.check()?;
    sqlx::query(
        "UPDATE users SET first_name = COALESCE($2, first_name), last_name = COALESCE($3, last_name) WHERE id = $1",
    )
    .bind(user.id)
    .bind(body.first_name.map(|s| s.trim().to_string()))
    .bind(body.last_name.map(|s| s.trim().to_string()))
    .execute(&state.db)
    .await?;
    let me = queries::me(&state.db, user.id).await?;
    Ok(Json(json!({ "user": me })))
}
