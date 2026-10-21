//! SQL shared by several route modules: Me, schools, places with stats, reviews.

use std::collections::{BTreeMap, HashMap};

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{
    first_sentence, Author, Me, PendingSchoolChange, Place, PlaceDetail, PlaceRow, Quote, ReviewRow, School,
    SchoolSummary, User,
};

pub async fn school_summary(db: &PgPool, id: Option<Uuid>) -> AppResult<Option<SchoolSummary>> {
    let Some(id) = id else { return Ok(None) };
    Ok(
        sqlx::query_as::<_, SchoolSummary>("SELECT id, name, kind, city, state FROM schools WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

pub async fn school(db: &PgPool, id: Uuid) -> AppResult<Option<School>> {
    Ok(
        sqlx::query_as::<_, School>("SELECT id, name, kind, city, state, lat, lng FROM schools WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

pub async fn user_by_id(db: &PgPool, id: Uuid) -> AppResult<User> {
    Ok(sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await?)
}

/// Re-reads the user (so the response reflects the latest writes) and builds `Me`.
pub async fn me(db: &PgPool, user_id: Uuid) -> AppResult<Me> {
    let user = user_by_id(db, user_id).await?;
    let school = school_summary(db, user.school_id).await?;
    let change: Option<(String, Option<Uuid>, chrono::NaiveDate)> =
        sqlx::query_as("SELECT email, school_id, enrolled_on FROM school_changes WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(db)
            .await?;
    let pending = match change {
        Some((email, change_school, enrolled_on)) => Some(PendingSchoolChange {
            email,
            needs_school: change_school.is_none(),
            school: school_summary(db, change_school).await?,
            enrolled_on: crate::enrollment::format_month(enrolled_on),
        }),
        None => None,
    };
    Ok(Me::new(&user, school, pending))
}

pub async fn author(db: &PgPool, user_id: Uuid) -> AppResult<Option<Author>> {
    let row: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT u.first_name, u.last_name, s.name, s.kind
         FROM users u LEFT JOIN schools s ON s.id = u.school_id WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|(f, l, sn, sk)| Author::new(&f, &l, sn, sk)))
}

// ---------- places ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceSort {
    Reviews,
    Rating,
    Distance,
    New,
    /// Most reviews in the last 30 days, then rating (feed "popular").
    Popular,
    /// Most recently saved first; requires `saved_by`.
    Saved,
}

#[derive(Clone, Debug)]
pub struct PlaceFilter {
    pub status: Option<String>,
    pub id: Option<Uuid>,
    pub q: Option<String>,
    pub category: Option<String>,
    pub kind: Option<String>,
    /// Facilities of this school plus businesses within 2 mi of its coordinates.
    pub school_area: Option<(Uuid, Option<(f64, f64)>)>,
    pub near: Option<(f64, f64)>,
    pub radius_mi: Option<f64>,
    pub deals: bool,
    /// Only places this user saved.
    pub saved_by: Option<Uuid>,
    pub sort: PlaceSort,
    pub limit: i64,
    pub offset: i64,
}

impl Default for PlaceFilter {
    fn default() -> Self {
        Self {
            status: Some("active".into()),
            id: None,
            q: None,
            category: None,
            kind: None,
            school_area: None,
            near: None,
            radius_mi: None,
            deals: false,
            saved_by: None,
            sort: PlaceSort::Reviews,
            limit: 20,
            offset: 0,
        }
    }
}

pub const SCHOOL_AREA_MI: f64 = 2.0;

/// Great-circle distance in miles between a row's coordinates and a bound point (no PostGIS).
pub fn push_haversine(qb: &mut QueryBuilder<'_, Postgres>, lat_col: &str, lng_col: &str, lat: f64, lng: f64) {
    qb.push("(7917.6 * asin(least(1.0, sqrt(power(sin(radians(")
        .push(lat_col)
        .push(" - ")
        .push_bind(lat)
        .push(") / 2), 2) + cos(radians(")
        .push_bind(lat)
        .push(")) * cos(radians(")
        .push(lat_col)
        .push(")) * power(sin(radians(")
        .push(lng_col)
        .push(" - ")
        .push_bind(lng)
        .push(") / 2), 2)))))");
}

/// Turns free text into a prefix tsquery: "gayley nood" -> "gayley:* & nood:*".
pub fn prefix_tsquery(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .take(8)
        .map(|t| format!("{}:*", t.to_lowercase()))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" & "))
}

pub async fn fetch_place_rows(db: &PgPool, f: &PlaceFilter) -> AppResult<Vec<PlaceRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT x.*, round(x.distance_raw::numeric, 1)::float8 AS distance_mi FROM (
           SELECT p.id, p.kind, p.name, p.category, p.subcategory, p.price_level, p.address, p.lat, p.lng,
                  p.school_id, p.student_discount, p.status, p.hours, p.created_by, p.created_at,
                  st.rating_avg, st.review_count, st.recent_count, ",
    );
    match f.saved_by {
        Some(uid) => {
            qb.push("(SELECT sp.created_at FROM saved_places sp WHERE sp.place_id = p.id AND sp.user_id = ")
                .push_bind(uid)
                .push(") AS saved_at, ");
        }
        None => {
            qb.push("NULL::timestamptz AS saved_at, ");
        }
    }
    match f.near {
        Some((lat, lng)) => push_haversine(&mut qb, "p.lat", "p.lng", lat, lng),
        None => {
            qb.push("NULL::float8");
        }
    }
    qb.push(
        " AS distance_raw
           FROM places p
           CROSS JOIN LATERAL (
             SELECT round(avg(r.rating)::numeric, 1)::float8 AS rating_avg,
                    count(*)::bigint AS review_count,
                    count(*) FILTER (WHERE r.created_at > now() - interval '30 days')::bigint AS recent_count
             FROM reviews r WHERE r.place_id = p.id AND r.deleted_at IS NULL
           ) st
           WHERE true",
    );
    if let Some(status) = &f.status {
        qb.push(" AND p.status = ").push_bind(status.clone());
    }
    if let Some(id) = f.id {
        qb.push(" AND p.id = ").push_bind(id);
    }
    if let Some(tsq) = f.q.as_deref().and_then(prefix_tsquery) {
        qb.push(" AND p.search @@ to_tsquery('english', ").push_bind(tsq).push(")");
    }
    if let Some(category) = &f.category {
        qb.push(" AND p.category = ").push_bind(category.clone());
    }
    if let Some(kind) = &f.kind {
        qb.push(" AND p.kind = ").push_bind(kind.clone());
    }
    if f.deals {
        qb.push(" AND p.student_discount IS NOT NULL");
    }
    if let Some(uid) = f.saved_by {
        qb.push(" AND EXISTS (SELECT 1 FROM saved_places sp WHERE sp.place_id = p.id AND sp.user_id = ")
            .push_bind(uid)
            .push(")");
    }
    if let Some((school_id, coords)) = f.school_area {
        qb.push(" AND (p.school_id = ").push_bind(school_id);
        if let Some((lat, lng)) = coords {
            qb.push(" OR (p.kind = 'business' AND ");
            push_haversine(&mut qb, "p.lat", "p.lng", lat, lng);
            qb.push(" <= ").push_bind(SCHOOL_AREA_MI).push(")");
        }
        qb.push(")");
    }
    qb.push(") x WHERE true");
    if f.near.is_some() {
        qb.push(" AND x.distance_raw <= ").push_bind(f.radius_mi.unwrap_or(1.0));
    }
    qb.push(match f.sort {
        PlaceSort::Reviews => " ORDER BY x.review_count DESC, x.rating_avg DESC NULLS LAST, x.name, x.id",
        PlaceSort::Rating => " ORDER BY x.rating_avg DESC NULLS LAST, x.review_count DESC, x.name, x.id",
        PlaceSort::Distance => " ORDER BY x.distance_raw ASC NULLS LAST, x.name, x.id",
        PlaceSort::New => " ORDER BY x.created_at DESC, x.id",
        PlaceSort::Saved => " ORDER BY x.saved_at DESC, x.id",
        PlaceSort::Popular => {
            " ORDER BY x.recent_count DESC, x.rating_avg DESC NULLS LAST, x.review_count DESC, x.name, x.id"
        }
    });
    qb.push(" LIMIT ").push_bind(f.limit).push(" OFFSET ").push_bind(f.offset);
    Ok(qb.build_query_as::<PlaceRow>().fetch_all(db).await?)
}

/// The first sentence of each place's most helpful review from the last 90 days.
pub async fn top_quotes(db: &PgPool, place_ids: &[Uuid]) -> AppResult<HashMap<Uuid, Quote>> {
    if place_ids.is_empty() {
        return Ok(HashMap::new());
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        place_id: Uuid,
        id: Uuid,
        body: String,
        first_name: String,
        last_name: String,
        school_name: Option<String>,
        school_kind: Option<String>,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT DISTINCT ON (t.place_id) t.* FROM (
           SELECT r.place_id, r.id, r.body, r.created_at, u.first_name, u.last_name,
                  s.name AS school_name, s.kind AS school_kind,
                  (SELECT count(*) FROM review_votes v WHERE v.review_id = r.id) AS helpful
           FROM reviews r
           JOIN users u ON u.id = r.user_id
           LEFT JOIN schools s ON s.id = r.school_id
           WHERE r.place_id = ANY($1) AND r.deleted_at IS NULL
             AND r.created_at > now() - interval '90 days'
         ) t
         ORDER BY t.place_id, t.helpful DESC, t.created_at DESC",
    )
    .bind(place_ids)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let quote = Quote {
                text: first_sentence(&r.body, 140),
                author: Author::new(&r.first_name, &r.last_name, r.school_name, r.school_kind),
                review_id: r.id,
            };
            (r.place_id, quote)
        })
        .collect())
}

pub async fn places_from_rows(db: &PgPool, rows: &[PlaceRow]) -> AppResult<Vec<Place>> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let mut quotes = top_quotes(db, &ids).await?;
    Ok(rows
        .iter()
        .map(|r| Place::from_row(r, quotes.remove(&r.id)))
        .collect())
}

/// Loads one place in any status; visibility is the caller's decision.
pub async fn place_row(db: &PgPool, id: Uuid) -> AppResult<Option<PlaceRow>> {
    let f = PlaceFilter {
        status: None,
        id: Some(id),
        limit: 1,
        ..Default::default()
    };
    Ok(fetch_place_rows(db, &f).await?.into_iter().next())
}

pub async fn place_detail(db: &PgPool, row: &PlaceRow, viewer: Option<Uuid>) -> AppResult<PlaceDetail> {
    let place = places_from_rows(db, std::slice::from_ref(row))
        .await?
        .pop()
        .expect("one place");
    let counts: Vec<(i16, i64)> = sqlx::query_as(
        "SELECT rating, count(*) FROM reviews WHERE place_id = $1 AND deleted_at IS NULL GROUP BY rating",
    )
    .bind(row.id)
    .fetch_all(db)
    .await?;
    let mut rating_distribution: BTreeMap<String, i64> = (1..=5).map(|n| (n.to_string(), 0)).collect();
    for (rating, n) in counts {
        rating_distribution.insert(rating.to_string(), n);
    }
    let viewer_review_id = match viewer {
        Some(uid) => {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM reviews WHERE place_id = $1 AND user_id = $2 AND deleted_at IS NULL",
            )
            .bind(row.id)
            .bind(uid)
            .fetch_optional(db)
            .await?
        }
        None => None,
    };
    let viewer_saved = match viewer {
        Some(uid) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM saved_places WHERE place_id = $1 AND user_id = $2)",
            )
            .bind(row.id)
            .bind(uid)
            .fetch_one(db)
            .await?
        }
        None => false,
    };
    Ok(PlaceDetail {
        place,
        viewer_saved,
        hours: row.hours.clone(),
        rating_distribution,
        school: school_summary(db, row.school_id).await?,
        viewer_review_id,
    })
}

// ---------- reviews ----------

/// SELECT list for `ReviewRow`; `$1` is the viewer id (may be NULL). The author's school is the
/// snapshot stored on the review, not their current one.
pub const REVIEW_SELECT: &str = "SELECT r.id, r.place_id, r.user_id, r.rating, r.body, r.visited_month, r.used_deal,
       r.created_at, r.updated_at, r.deleted_at, r.deleted_by, r.edited_at,
       (SELECT count(*) FROM review_votes v WHERE v.review_id = r.id) AS helpful_count,
       EXISTS (SELECT 1 FROM review_votes v WHERE v.review_id = r.id AND v.user_id = $1) AS viewer_voted,
       u.first_name, u.last_name, s.name AS school_name, s.kind AS school_kind
     FROM reviews r
     JOIN users u ON u.id = r.user_id
     LEFT JOIN schools s ON s.id = r.school_id";

/// Loads one review, including soft-deleted ones.
pub async fn review_row(db: &PgPool, id: Uuid, viewer: Option<Uuid>) -> AppResult<Option<ReviewRow>> {
    Ok(
        sqlx::query_as::<_, ReviewRow>(&format!("{REVIEW_SELECT} WHERE r.id = $2"))
            .bind(viewer)
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}
