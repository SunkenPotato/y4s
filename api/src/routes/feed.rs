use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::MaybeUser;
use crate::error::{AppError, AppResult};
use crate::extract::ApiQuery;
use crate::models::School;
use crate::queries::{self, PlaceFilter, PlaceSort};
use crate::AppState;

#[derive(Deserialize)]
pub struct FeedQuery {
    school_id: Option<Uuid>,
}

pub async fn get_feed(
    State(state): State<AppState>,
    MaybeUser(viewer): MaybeUser,
    ApiQuery(params): ApiQuery<FeedQuery>,
) -> AppResult<Json<Value>> {
    let school_id = match params.school_id {
        Some(id) => Some(id),
        None => match viewer.as_ref().and_then(|u| u.school_id) {
            Some(id) => Some(id),
            None => {
                sqlx::query_scalar::<_, Uuid>("SELECT id FROM schools ORDER BY created_at, id LIMIT 1")
                    .fetch_optional(&state.db)
                    .await?
            }
        },
    };
    let school: School = match school_id {
        Some(id) => queries::school(&state.db, id).await?.ok_or_else(AppError::not_found)?,
        None => return Err(AppError::not_found()),
    };
    let coords = school.lat.zip(school.lng);
    let area = Some((school.id, coords));

    let popular_rows = queries::fetch_place_rows(
        &state.db,
        &PlaceFilter {
            school_area: area,
            sort: PlaceSort::Popular,
            limit: 8,
            ..Default::default()
        },
    )
    .await?;
    let popular = queries::places_from_rows(&state.db, &popular_rows).await?;

    let deal_rows = queries::fetch_place_rows(
        &state.db,
        &PlaceFilter {
            school_area: area,
            deals: true,
            sort: PlaceSort::Reviews,
            limit: 6,
            ..Default::default()
        },
    )
    .await?;
    let deals = queries::places_from_rows(&state.db, &deal_rows).await?;

    // The hero is the most popular place that has a recent quotable review.
    let featured = popular.iter().find_map(|p| {
        p.top_quote
            .as_ref()
            .map(|q| json!({ "place": p, "quote": q }))
    });

    Ok(Json(json!({
        "school": school,
        "featured": featured,
        "popular": popular,
        "deals": deals,
    })))
}
