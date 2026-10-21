use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::domains;
use crate::error::{AppError, AppResult};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::mail;
use crate::models::{Author, SchoolSummary};
use crate::pagination::PageReq;
use crate::queries::{self, PlaceFilter, PlaceSort};
use crate::AppState;

// ---------- domains ----------

#[derive(Deserialize)]
pub struct StatusQuery {
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct DomainItem {
    domain: String,
    status: String,
    school: SchoolSummary,
    waiting_count: i64,
    requested_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
    flags: Vec<&'static str>,
}

#[derive(sqlx::FromRow)]
struct DomainRow {
    domain: String,
    status: String,
    requested_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
    waiting_count: i64,
    school_id: Uuid,
    school_name: String,
    school_kind: String,
    school_city: String,
    school_state: String,
}

const DOMAIN_SELECT: &str = "SELECT d.domain, d.status, d.requested_at, d.decided_at,
       (SELECT count(*) FROM users u WHERE u.email_domain = d.domain
          AND u.status IN ('pending_school', 'rejected')) AS waiting_count,
       s.id AS school_id, s.name AS school_name, s.kind AS school_kind, s.city AS school_city, s.state AS school_state
     FROM school_domains d JOIN schools s ON s.id = d.school_id";

impl From<DomainRow> for DomainItem {
    fn from(r: DomainRow) -> Self {
        let mut flags = Vec::new();
        if domains::is_personal_provider(&r.domain) {
            flags.push("personal_provider");
        }
        Self {
            school: SchoolSummary {
                id: r.school_id,
                name: r.school_name,
                kind: r.school_kind,
                city: r.school_city,
                state: r.school_state,
            },
            domain: r.domain,
            status: r.status,
            waiting_count: r.waiting_count,
            requested_at: r.requested_at,
            decided_at: r.decided_at,
            flags,
        }
    }
}

async fn domain_item(state: &AppState, domain: &str) -> AppResult<DomainItem> {
    let row = sqlx::query_as::<_, DomainRow>(&format!("{DOMAIN_SELECT} WHERE d.domain = $1"))
        .bind(domain)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(row.into())
}

pub async fn list_domains(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    ApiQuery(q): ApiQuery<StatusQuery>,
) -> AppResult<Json<Value>> {
    let status = q.status.unwrap_or_else(|| "pending".into());
    if !matches!(status.as_str(), "pending" | "approved" | "rejected") {
        return Err(AppError::field("status", "Use pending, approved or rejected."));
    }
    let rows = sqlx::query_as::<_, DomainRow>(&format!(
        "{DOMAIN_SELECT} WHERE d.status = $1 ORDER BY d.requested_at DESC, d.domain"
    ))
    .bind(&status)
    .fetch_all(&state.db)
    .await?;
    let items: Vec<DomainItem> = rows.into_iter().map(Into::into).collect();
    Ok(Json(json!({ "items": items })))
}

/// Locks the domain row and returns `(school_id, previous decided_at)`.
async fn lock_domain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    domain: &str,
) -> AppResult<(Uuid, Option<DateTime<Utc>>)> {
    sqlx::query_as("SELECT school_id, decided_at FROM school_domains WHERE domain = $1 FOR UPDATE")
        .bind(domain)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(AppError::not_found)
}

pub async fn approve_domain(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiPath(domain): ApiPath<String>,
) -> AppResult<Json<DomainItem>> {
    let domain = domain.to_lowercase();
    let mut tx = state.db.begin().await?;
    let (school_id, _) = lock_domain(&mut tx, &domain).await?;
    let decided_at: DateTime<Utc> = sqlx::query_scalar(
        "UPDATE school_domains SET status = 'approved', decided_by = $2, decided_at = now(), reject_reason = NULL
         WHERE domain = $1 RETURNING decided_at",
    )
    .bind(&domain)
    .bind(admin.id)
    .fetch_one(&mut *tx)
    .await?;
    // Only users who confirmed their email move to verified; the rest get there when they enter their code.
    let notified: Vec<(String, String)> = sqlx::query_as(
        "UPDATE users SET status = 'verified', school_id = $2, domain_decided_at = $3,
                role = CASE WHEN email = ANY($4) THEN 'admin' ELSE role END
         WHERE email_domain = $1 AND status IN ('pending_school', 'rejected') AND verified_at IS NOT NULL
         RETURNING email, first_name",
    )
    .bind(&domain)
    .bind(school_id)
    .bind(decided_at)
    .bind(&state.config.admin_emails)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let school = queries::school_summary(&state.db, Some(school_id))
        .await?
        .map(|s| s.name)
        .unwrap_or_default();
    for (email, first) in notified {
        mail::send_logged(&state.mailer, mail::approved_email(&email, &first, &school)).await;
    }
    Ok(Json(domain_item(&state, &domain).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct RejectBody {
    reason: Option<String>,
}

pub async fn reject_domain(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiPath(domain): ApiPath<String>,
    ApiJson(body): ApiJson<RejectBody>,
) -> AppResult<Json<DomainItem>> {
    let domain = domain.to_lowercase();
    let reason = body.reason.map(|r| r.trim().to_string()).filter(|r| !r.is_empty());
    if reason.as_ref().is_some_and(|r| r.chars().count() > 500) {
        return Err(AppError::field("reason", "Keep the reason under 500 characters."));
    }
    let mut tx = state.db.begin().await?;
    lock_domain(&mut tx, &domain).await?;
    let decided_at: DateTime<Utc> = sqlx::query_scalar(
        "UPDATE school_domains SET status = 'rejected', decided_by = $2, decided_at = now(), reject_reason = $3
         WHERE domain = $1 RETURNING decided_at",
    )
    .bind(&domain)
    .bind(admin.id)
    .bind(&reason)
    .fetch_one(&mut *tx)
    .await?;
    let notified: Vec<(String, String)> = sqlx::query_as(
        "UPDATE users SET status = 'rejected', domain_decided_at = $2
         WHERE email_domain = $1 AND status = 'pending_school' AND verified_at IS NOT NULL
         RETURNING email, first_name",
    )
    .bind(&domain)
    .bind(decided_at)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    for (email, first) in notified {
        mail::send_logged(&state.mailer, mail::rejected_email(&email, &first, reason.as_deref())).await;
    }
    Ok(Json(domain_item(&state, &domain).await?))
}

/// Undo: the domain goes back to pending, and exactly the users moved by the last decision go back
/// to `pending_school`.
pub async fn reset_domain(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    ApiPath(domain): ApiPath<String>,
) -> AppResult<Json<DomainItem>> {
    let domain = domain.to_lowercase();
    let mut tx = state.db.begin().await?;
    let (_, previous) = lock_domain(&mut tx, &domain).await?;
    sqlx::query(
        "UPDATE school_domains SET status = 'pending', decided_by = NULL, decided_at = NULL, reject_reason = NULL
         WHERE domain = $1",
    )
    .bind(&domain)
    .execute(&mut *tx)
    .await?;
    if let Some(previous) = previous {
        sqlx::query(
            "UPDATE users SET status = 'pending_school', domain_decided_at = NULL
             WHERE email_domain = $1 AND domain_decided_at = $2 AND status IN ('verified', 'rejected')",
        )
        .bind(&domain)
        .bind(previous)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(domain_item(&state, &domain).await?))
}

// ---------- places ----------

async fn admin_place_json(state: &AppState, id: Uuid, admin: Uuid) -> AppResult<Value> {
    let row = queries::place_row(&state.db, id).await?.ok_or_else(AppError::not_found)?;
    let detail = queries::place_detail(&state.db, &row, Some(admin)).await?;
    let mut value = serde_json::to_value(detail).map_err(anyhow::Error::from)?;
    let created_by: Option<Author> = match row.created_by {
        Some(uid) => queries::author(&state.db, uid).await?,
        None => None,
    };
    value["created_by"] = json!(created_by);
    Ok(value)
}

pub async fn list_places(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiQuery(q): ApiQuery<StatusQuery>,
) -> AppResult<Json<Value>> {
    let page = PageReq::parse(q.limit, q.cursor.as_deref())?;
    let status = q.status.unwrap_or_else(|| "pending".into());
    if !matches!(status.as_str(), "pending" | "active" | "hidden") {
        return Err(AppError::field("status", "Use pending, active or hidden."));
    }
    let rows = queries::fetch_place_rows(
        &state.db,
        &PlaceFilter {
            status: Some(status),
            sort: PlaceSort::New,
            limit: page.fetch_limit(),
            offset: page.offset,
            ..Default::default()
        },
    )
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        items.push(admin_place_json(&state, row.id, admin.id).await?);
    }
    Ok(Json(json!(page.finish(items))))
}

async fn set_place_status(state: &AppState, id: Uuid, status: &str, admin: Uuid) -> AppResult<Json<Value>> {
    let updated = sqlx::query("UPDATE places SET status = $2 WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(&state.db)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::not_found());
    }
    let row = queries::place_row(&state.db, id).await?.ok_or_else(AppError::not_found)?;
    let detail = queries::place_detail(&state.db, &row, Some(admin)).await?;
    Ok(Json(json!({ "place": detail })))
}

pub async fn approve_place(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    set_place_status(&state, id, "active", admin.id).await
}

pub async fn hide_place(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiPath(id): ApiPath<Uuid>,
) -> AppResult<Json<Value>> {
    set_place_status(&state, id, "hidden", admin.id).await
}

// ---------- reports ----------

#[derive(sqlx::FromRow)]
struct ReportRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    target_preview: Option<String>,
    reason: String,
    details: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    first_name: String,
    last_name: String,
    school_name: Option<String>,
    school_kind: Option<String>,
    report_count: i64,
}

const REPORT_SELECT: &str = "SELECT r.id, r.target_type, r.target_id, r.reason, r.details, r.status, r.created_at,
       u.first_name, u.last_name, s.name AS school_name, s.kind AS school_kind,
       CASE WHEN r.target_type = 'review'
            THEN (SELECT left(rv.body, 140) FROM reviews rv WHERE rv.id = r.target_id)
            ELSE (SELECT p.name FROM places p WHERE p.id = r.target_id) END AS target_preview,
       (SELECT count(*) FROM reports r2 WHERE r2.target_type = r.target_type AND r2.target_id = r.target_id
          AND r2.status = r.status) AS report_count
     FROM reports r
     JOIN users u ON u.id = r.reporter_id
     LEFT JOIN schools s ON s.id = u.school_id";

fn report_json(r: ReportRow) -> Value {
    json!({
        "id": r.id,
        "target_type": r.target_type,
        "target_id": r.target_id,
        "target_preview": r.target_preview.unwrap_or_default(),
        "reason": r.reason,
        "details": r.details,
        "reporter": Author::new(&r.first_name, &r.last_name, r.school_name, r.school_kind),
        "report_count": r.report_count,
        "status": r.status,
        "created_at": r.created_at,
    })
}

pub async fn list_reports(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    ApiQuery(q): ApiQuery<StatusQuery>,
) -> AppResult<Json<Value>> {
    let page = PageReq::parse(q.limit, q.cursor.as_deref())?;
    let status = q.status.unwrap_or_else(|| "open".into());
    if !matches!(status.as_str(), "open" | "resolved") {
        return Err(AppError::field("status", "Use open or resolved."));
    }
    let rows = sqlx::query_as::<_, ReportRow>(&format!(
        "{REPORT_SELECT} WHERE r.status = $1 ORDER BY r.created_at DESC, r.id LIMIT $2 OFFSET $3"
    ))
    .bind(&status)
    .bind(page.fetch_limit())
    .bind(page.offset)
    .fetch_all(&state.db)
    .await?;
    let items: Vec<Value> = rows.into_iter().map(report_json).collect();
    Ok(Json(json!(page.finish(items))))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ResolveBody {
    action: String,
}

pub async fn resolve_report(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<ResolveBody>,
) -> AppResult<Json<Value>> {
    if !matches!(body.action.as_str(), "dismiss" | "remove_content") {
        return Err(AppError::field("action", "Choose dismiss or remove_content."));
    }
    let mut tx = state.db.begin().await?;
    let (target_type, target_id): (String, Uuid) =
        sqlx::query_as("SELECT target_type, target_id FROM reports WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(AppError::not_found)?;
    if body.action == "dismiss" {
        sqlx::query(
            "UPDATE reports SET status = 'resolved', resolution = 'dismiss', resolved_by = $2, resolved_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(admin.id)
        .execute(&mut *tx)
        .await?;
    } else {
        if target_type == "place" {
            sqlx::query("UPDATE places SET status = 'hidden' WHERE id = $1")
                .bind(target_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE reviews SET deleted_at = now(), deleted_by = $2 WHERE id = $1 AND deleted_at IS NULL")
                .bind(target_id)
                .bind(admin.id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query(
            "UPDATE reports SET status = 'resolved', resolution = 'remove_content', resolved_by = $3, resolved_at = now()
             WHERE (id = $4) OR (target_type = $1 AND target_id = $2 AND status = 'open')",
        )
        .bind(&target_type)
        .bind(target_id)
        .bind(admin.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    let row = sqlx::query_as::<_, ReportRow>(&format!("{REPORT_SELECT} WHERE r.id = $1"))
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(json!({ "report": report_json(row) })))
}
