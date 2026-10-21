use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder};

use crate::db::escape_like;
use crate::error::{AppError, AppResult};
use crate::extract::ApiQuery;
use crate::models::School;
use crate::AppState;

#[derive(Deserialize)]
pub struct SchoolsQuery {
    q: Option<String>,
    kind: Option<String>,
    limit: Option<i64>,
}

/// Case-insensitive match on the start of the name or the start of any word in it.
pub async fn list(State(state): State<AppState>, ApiQuery(params): ApiQuery<SchoolsQuery>) -> AppResult<Json<Value>> {
    let limit = params.limit.unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(AppError::field("limit", "Use a limit from 1 to 50."));
    }
    if let Some(kind) = &params.kind {
        if !matches!(kind.as_str(), "university" | "high_school") {
            return Err(AppError::field("kind", "Use university or high_school."));
        }
    }
    let q = params.q.as_deref().map(str::trim).filter(|q| !q.is_empty());

    let mut qb = QueryBuilder::<Postgres>::new("SELECT id, name, kind, city, state, lat, lng FROM schools WHERE true");
    if let Some(kind) = &params.kind {
        qb.push(" AND kind = ").push_bind(kind.clone());
    }
    if let Some(q) = q {
        let prefix = format!("{}%", escape_like(&q.to_lowercase()));
        let word = format!("% {}%", escape_like(&q.to_lowercase()));
        qb.push(" AND (lower(name) LIKE ")
            .push_bind(prefix.clone())
            .push(" OR lower(name) LIKE ")
            .push_bind(word)
            .push(") ORDER BY (lower(name) LIKE ")
            .push_bind(prefix)
            .push(") DESC, name");
    } else {
        qb.push(" ORDER BY name");
    }
    qb.push(" LIMIT ").push_bind(limit);
    let items = qb.build_query_as::<School>().fetch_all(&state.db).await?;
    Ok(Json(json!({ "items": items })))
}
