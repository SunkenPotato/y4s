use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::auth::{check_write_limit, MaybeUser, VerifiedUser};
use crate::error::{AppError, AppResult, FieldErrors};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::models::{PlaceRow, User, CATEGORIES};
use crate::pagination::PageReq;
use crate::queries::{self, PlaceFilter, PlaceSort};
use crate::AppState;

/// Two places with the same normalized name this close together are treated as duplicates.
// Geocoders disagree by a few hundred meters on the same address, and two
// branches of one name this close together are rare, so 400 m catches real duplicates.
const DUPLICATE_RADIUS_MI: f64 = 400.0 / 1609.344;

#[derive(Deserialize)]
pub struct PlacesQuery {
    q: Option<String>,
    category: Option<String>,
    kind: Option<String>,
    school_id: Option<Uuid>,
    near: Option<String>,
    radius_mi: Option<f64>,
    deals: Option<bool>,
    sort: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

fn parse_near(raw: &str) -> Option<(f64, f64)> {
    let (lat, lng) = raw.split_once(',')?;
    let lat: f64 = lat.trim().parse().ok()?;
    let lng: f64 = lng.trim().parse().ok()?;
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng)).then_some((lat, lng))
}

pub async fn list(State(state): State<AppState>, ApiQuery(p): ApiQuery<PlacesQuery>) -> AppResult<Json<Value>> {
    let page = PageReq::parse(p.limit, p.cursor.as_deref())?;
    let mut errors = FieldErrors::default();
    if let Some(c) = &p.category {
        if !CATEGORIES.contains(&c.as_str()) {
            errors.add("category", "Pick one of the listed categories.");
        }
    }
    if let Some(k) = &p.kind {
        if !matches!(k.as_str(), "business" | "facility") {
            errors.add("kind", "Use business or facility.");
        }
    }
    let near = match p.near.as_deref() {
        Some(raw) => {
            let parsed = parse_near(raw);
            if parsed.is_none() {
                errors.add("near", "Use near=lat,lng, like near=34.0689,-118.4452.");
            }
            parsed
        }
        None => None,
    };
    if let Some(r) = p.radius_mi {
        if !(r > 0.0 && r <= 10.0) {
            errors.add("radius_mi", "Use a radius from 0 to 10 miles.");
        }
    }
    let sort = match p.sort.as_deref().unwrap_or("reviews") {
        "reviews" => PlaceSort::Reviews,
        "rating" => PlaceSort::Rating,
        "new" => PlaceSort::New,
        "distance" => {
            if p.near.is_none() {
                errors.add("sort", "Sorting by distance needs a location (near=lat,lng).");
            }
            PlaceSort::Distance
        }
        _ => {
            errors.add("sort", "Sort by reviews, rating, distance or new.");
            PlaceSort::Reviews
        }
    };
    errors.check()?;

    let school_area = match p.school_id {
        Some(id) => {
            let school = queries::school(&state.db, id).await?.ok_or_else(AppError::not_found)?;
            Some((school.id, school.lat.zip(school.lng)))
        }
        None => None,
    };
    let filter = PlaceFilter {
        q: p.q.filter(|q| !q.trim().is_empty()),
        category: p.category,
        kind: p.kind,
        school_area,
        near,
        radius_mi: p.radius_mi,
        deals: p.deals.unwrap_or(false),
        sort,
        limit: page.fetch_limit(),
        offset: page.offset,
        ..Default::default()
    };
    let rows = queries::fetch_place_rows(&state.db, &filter).await?;
    let places = queries::places_from_rows(&state.db, &rows).await?;
    Ok(Json(json!(page.finish(places))))
}

/// Active places are public; pending ones only for their creator and admins; hidden ones only for admins.
pub async fn visible_place(state: &AppState, id: Uuid, viewer: Option<&User>) -> AppResult<PlaceRow> {
    let row = queries::place_row(&state.db, id).await?.ok_or_else(AppError::not_found)?;
    let is_admin = viewer.is_some_and(|u| u.role == "admin");
    let is_creator = viewer.is_some_and(|u| row.created_by == Some(u.id));
    let visible = match row.status.as_str() {
        "active" => true,
        "pending" => is_admin || is_creator,
        _ => is_admin,
    };
    if visible {
        Ok(row)
    } else {
        Err(AppError::not_found())
    }
}

pub async fn get_place(
    State(state): State<AppState>,
    MaybeUser(viewer): MaybeUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    let row = visible_place(&state, id, viewer.as_ref()).await?;
    let detail = queries::place_detail(&state.db, &row, viewer.map(|u| u.id)).await?;
    Ok(Json(json!({ "place": detail })))
}

#[derive(Deserialize, Serialize)]
pub struct HoursEntry {
    days: String,
    time: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct CreatePlace {
    kind: String,
    name: String,
    category: String,
    subcategory: Option<String>,
    address: String,
    lat: Option<f64>,
    lng: Option<f64>,
    school_id: Option<Uuid>,
    price_level: Option<i16>,
    student_discount: Option<String>,
    hours: Option<Vec<HoursEntry>>,
}

fn trimmed_opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub async fn create(
    State(state): State<AppState>,
    VerifiedUser(user): VerifiedUser,
    ApiJson(body): ApiJson<CreatePlace>,
) -> AppResult<(StatusCode, Json<Value>)> {
    check_write_limit(&state, &user)?;
    let name = body.name.trim().to_string();
    let address = body.address.trim().to_string();
    let subcategory = trimmed_opt(body.subcategory);
    let student_discount = trimmed_opt(body.student_discount);

    let mut errors = FieldErrors::default();
    if !matches!(body.kind.as_str(), "business" | "facility") {
        errors.add("kind", "Pick business or campus facility.");
    }
    let name_len = name.chars().count();
    if name_len == 0 {
        errors.add("name", "Enter the place's name.");
    } else if name_len > 80 {
        errors.add("name", "Use 80 characters or fewer.");
    }
    if !CATEGORIES.contains(&body.category.as_str()) {
        errors.add("category", "Pick a category.");
    }
    if subcategory.as_ref().is_some_and(|s| s.chars().count() > 40) {
        errors.add("subcategory", "Use 40 characters or fewer.");
    }
    let addr_len = address.chars().count();
    if addr_len < 5 {
        errors.add("address", "Enter the street address, like 1062 Gayley Ave, Los Angeles, CA.");
    } else if addr_len > 200 {
        errors.add("address", "Use 200 characters or fewer.");
    }
    match (body.lat, body.lng) {
        (Some(lat), Some(lng)) => {
            if !(-90.0..=90.0).contains(&lat) {
                errors.add("lat", "Use a latitude between -90 and 90.");
            }
            if !(-180.0..=180.0).contains(&lng) {
                errors.add("lng", "Use a longitude between -180 and 180.");
            }
        }
        (None, None) => {}
        _ => errors.add("lat", "Send both lat and lng, or neither."),
    }
    if body.price_level.is_some_and(|p| !(1..=4).contains(&p)) {
        errors.add("price_level", "Pick a price level from $ to $$$$.");
    }
    if student_discount.as_ref().is_some_and(|s| s.chars().count() > 80) {
        errors.add("student_discount", "Use 80 characters or fewer.");
    }
    if let Some(hours) = &body.hours {
        let bad = hours.len() > 14
            || hours.iter().any(|h| {
                let (d, t) = (h.days.trim().chars().count(), h.time.trim().chars().count());
                d == 0 || d > 40 || t == 0 || t > 40
            });
        if bad {
            errors.add("hours", "Add up to 14 rows, each with days and times.");
        }
    }
    if body.kind == "facility" && body.school_id.is_none() {
        errors.add("school_id", "Campus facilities need a school.");
    }
    errors.check()?;
    if let Some(sid) = body.school_id {
        if queries::school(&state.db, sid).await?.is_none() {
            return Err(AppError::field("school_id", "Pick a school from the list."));
        }
    }

    let (lat, lng) = match body.lat.zip(body.lng) {
        Some(c) => c,
        None => match state.geocoder.geocode(&address).await {
            Ok(Some(c)) => c,
            Ok(None) | Err(_) => {
                let mut fields = std::collections::BTreeMap::new();
                fields.insert(
                    "address".to_string(),
                    "We couldn't find this address. Check the street and city.".to_string(),
                );
                return Err(AppError::unprocessable(
                    "geocode_failed",
                    "We couldn't find this address. Check the street and city.",
                )
                .with_fields(fields));
            }
        },
    };

    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT id FROM places WHERE status <> 'hidden' AND name_norm = regexp_replace(lower(",
    );
    qb.push_bind(&name).push("), '[^a-z0-9]', '', 'g') AND ");
    queries::push_haversine(&mut qb, "lat", "lng", lat, lng);
    qb.push(" <= ").push_bind(DUPLICATE_RADIUS_MI).push(" LIMIT 1");
    if let Some(existing) = qb.build_query_scalar::<Uuid>().fetch_optional(&state.db).await? {
        return Err(AppError::conflict(
            "duplicate_place",
            "This place is already on Corner.",
        )
        .with("existing_id", existing.to_string()));
    }

    let id = Uuid::new_v4();
    let hours = body.hours.map(|h| {
        json!(h
            .into_iter()
            .map(|e| HoursEntry { days: e.days.trim().into(), time: e.time.trim().into() })
            .collect::<Vec<_>>())
    });
    sqlx::query(
        "INSERT INTO places (id, kind, name, category, subcategory, price_level, address, lat, lng, school_id,
                             student_discount, hours, status, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'pending', $13)",
    )
    .bind(id)
    .bind(&body.kind)
    .bind(&name)
    .bind(&body.category)
    .bind(subcategory)
    .bind(body.price_level)
    .bind(&address)
    .bind(lat)
    .bind(lng)
    .bind(body.school_id)
    .bind(student_discount)
    .bind(hours)
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let row = queries::place_row(&state.db, id).await?.ok_or_else(AppError::internal)?;
    let detail = queries::place_detail(&state.db, &row, Some(user.id)).await?;
    Ok((StatusCode::CREATED, Json(json!({ "place": detail }))))
}
