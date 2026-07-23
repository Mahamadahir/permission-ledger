use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use url::Url;
use uuid::Uuid;

use crate::{
    audit, auth,
    error::{ApiError, ApiResult},
    AppState,
};

#[derive(Deserialize)]
pub struct RecordRequest {
    service_name: String,
    website_url: String,
    consent_type: String,
    category_id: Uuid,
    date_given: NaiveDate,
    review_date: Option<NaiveDate>,
    expiry_date: Option<NaiveDate>,
    status: Option<String>,
    risk_level: String,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub struct RecordFilters {
    pub q: Option<String>,
    pub category_id: Option<Uuid>,
    pub status: Option<String>,
    pub risk_level: Option<String>,
    pub source: Option<String>,
    pub review_due: Option<bool>,
    pub expired: Option<bool>,
    pub sort: Option<String>,
}

#[derive(Serialize, FromRow, Clone)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub sort_order: i32,
}

#[derive(Serialize, FromRow, Clone)]
pub struct RecordResponse {
    pub id: Uuid,
    pub service_id: Uuid,
    pub service_name: String,
    pub normalized_domain: String,
    pub website_url: String,
    pub category_id: Uuid,
    pub category_name: String,
    pub consent_type: String,
    pub date_given: NaiveDate,
    pub review_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub risk_level: String,
    pub source: String,
    pub notes: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
pub struct SummaryResponse {
    active: i64,
    review_due: i64,
    expired: i64,
    revoked: i64,
    high_risk: i64,
}

#[derive(Serialize, FromRow)]
pub struct NamedCount {
    name: String,
    count: i64,
}

#[derive(Serialize)]
pub struct DashboardResponse {
    summary: SummaryResponse,
    recent: Vec<RecordResponse>,
    categories: Vec<NamedCount>,
    services: Vec<NamedCount>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(categories))
        .route("/records", get(list_records).post(create_record))
        .route(
            "/records/:id",
            get(get_record).put(update_record).delete(delete_record),
        )
        .route("/records/:id/revoke", post(revoke_record))
        .route("/dashboard", get(dashboard))
}

pub async fn categories(State(state): State<AppState>) -> ApiResult<Json<Vec<CategoryResponse>>> {
    let rows = sqlx::query_as::<_, CategoryResponse>(
        "SELECT id, slug, name, sort_order FROM consent_categories ORDER BY sort_order, name",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn list_records(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(filters): Query<RecordFilters>,
) -> ApiResult<Json<Vec<RecordResponse>>> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let rows = fetch_records(&state.pool, user.id, filters, 200).await?;
    Ok(Json(rows))
}

async fn get_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RecordResponse>> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let row = fetch_record(&state.pool, user.id, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

async fn create_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecordRequest>,
) -> ApiResult<Json<RecordResponse>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    let record = create_record_for_user(&state.pool, user.id, payload, "manual").await?;
    audit::log_event(
        &state.pool,
        Some(user.id),
        "consent.created",
        Some("consent_record"),
        Some(record.id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(record))
}

async fn update_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<RecordRequest>,
) -> ApiResult<Json<RecordResponse>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    ensure_record_owner(&state.pool, user.id, id).await?;
    let normalized = normalize_service(&payload.website_url)?;
    let service_name = clean_required(&payload.service_name, "service name", 160)?;
    let website_url = clean_required(&payload.website_url, "website URL", 500)?;
    let service_id = upsert_service(&state.pool, &normalized).await?;
    validate_category(&state.pool, payload.category_id).await?;
    let status = validate_status(payload.status.as_deref().unwrap_or("active"))?;
    let risk_level = validate_risk(&payload.risk_level)?;

    sqlx::query(
        r#"
        UPDATE consent_records
        SET service_id = $1, service_name = $2, website_url = $3, category_id = $4,
            consent_type = $5, date_given = $6, review_date = $7, expiry_date = $8,
            status = $9, risk_level = $10, notes = $11, updated_at = now()
        WHERE id = $12 AND user_id = $13
        "#,
    )
    .bind(service_id)
    .bind(service_name)
    .bind(website_url)
    .bind(payload.category_id)
    .bind(clean_required(&payload.consent_type, "consent type", 160)?)
    .bind(payload.date_given)
    .bind(payload.review_date)
    .bind(payload.expiry_date)
    .bind(status)
    .bind(risk_level)
    .bind(clean_optional(payload.notes, 2000))
    .bind(id)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    audit::log_event(
        &state.pool,
        Some(user.id),
        "consent.updated",
        Some("consent_record"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(
        fetch_record(&state.pool, user.id, id)
            .await?
            .ok_or(ApiError::NotFound)?,
    ))
}

async fn revoke_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RecordResponse>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    ensure_record_owner(&state.pool, user.id, id).await?;
    sqlx::query("UPDATE consent_records SET status = 'revoked', revoked_at = now(), updated_at = now() WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    audit::log_event(
        &state.pool,
        Some(user.id),
        "consent.revoked",
        Some("consent_record"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(
        fetch_record(&state.pool, user.id, id)
            .await?
            .ok_or(ApiError::NotFound)?,
    ))
}

async fn delete_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    ensure_record_owner(&state.pool, user.id, id).await?;
    sqlx::query("DELETE FROM consent_records WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    audit::log_event(
        &state.pool,
        Some(user.id),
        "consent.deleted",
        Some("consent_record"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<DashboardResponse>> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let summary = sqlx::query_as::<_, SummaryResponse>(
        r#"
        SELECT
            count(*) FILTER (WHERE status = 'active') AS active,
            count(*) FILTER (WHERE (review_date IS NOT NULL AND review_date <= current_date) OR status = 'needs_review') AS review_due,
            count(*) FILTER (WHERE (expiry_date IS NOT NULL AND expiry_date <= current_date) OR status = 'expired') AS expired,
            count(*) FILTER (WHERE status = 'revoked') AS revoked,
            count(*) FILTER (WHERE risk_level = 'high') AS high_risk
        FROM consent_records
        WHERE user_id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    let recent = fetch_records(
        &state.pool,
        user.id,
        RecordFilters {
            q: None,
            category_id: None,
            status: None,
            risk_level: None,
            source: None,
            review_due: None,
            expired: None,
            sort: Some("created_desc".to_string()),
        },
        8,
    )
    .await?;
    let categories = sqlx::query_as::<_, NamedCount>(
        r#"
        SELECT consent_categories.name, count(*) AS count
        FROM consent_records
        JOIN consent_categories ON consent_categories.id = consent_records.category_id
        WHERE consent_records.user_id = $1
        GROUP BY consent_categories.name
        ORDER BY count DESC, consent_categories.name
        LIMIT 8
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let services = sqlx::query_as::<_, NamedCount>(
        r#"
        SELECT consent_records.service_name AS name, count(*) AS count
        FROM consent_records
        WHERE consent_records.user_id = $1
        GROUP BY consent_records.service_name
        ORDER BY count DESC, consent_records.service_name
        LIMIT 8
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(DashboardResponse {
        summary,
        recent,
        categories,
        services,
    }))
}

pub async fn create_record_for_user(
    pool: &PgPool,
    user_id: Uuid,
    payload: RecordRequest,
    source: &str,
) -> ApiResult<RecordResponse> {
    let normalized = normalize_service(&payload.website_url)?;
    let service_name = clean_required(&payload.service_name, "service name", 160)?;
    let website_url = clean_required(&payload.website_url, "website URL", 500)?;
    let service_id = upsert_service(pool, &normalized).await?;
    validate_category(pool, payload.category_id).await?;
    let status = validate_status(payload.status.as_deref().unwrap_or("active"))?;
    let risk_level = validate_risk(&payload.risk_level)?;
    let source = validate_source(source)?;

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO consent_records (
            user_id, service_id, service_name, website_url, category_id, consent_type,
            date_given, review_date, expiry_date, status, risk_level, source, notes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(service_id)
    .bind(service_name)
    .bind(website_url)
    .bind(payload.category_id)
    .bind(clean_required(&payload.consent_type, "consent type", 160)?)
    .bind(payload.date_given)
    .bind(payload.review_date)
    .bind(payload.expiry_date)
    .bind(status)
    .bind(risk_level)
    .bind(source)
    .bind(clean_optional(payload.notes, 2000))
    .fetch_one(pool)
    .await?;

    fetch_record(pool, user_id, id)
        .await?
        .ok_or(ApiError::NotFound)
}

pub async fn fetch_records(
    pool: &PgPool,
    user_id: Uuid,
    filters: RecordFilters,
    limit: i64,
) -> ApiResult<Vec<RecordResponse>> {
    let mut query = QueryBuilder::<Postgres>::new(record_select_sql());
    query
        .push(" WHERE consent_records.user_id = ")
        .push_bind(user_id);

    if let Some(q) = clean_optional(filters.q, 200) {
        let pattern = format!("%{}%", q.to_lowercase());
        query
            .push(" AND (lower(consent_records.service_name) LIKE ")
            .push_bind(pattern.clone())
            .push(" OR lower(consent_records.website_url) LIKE ")
            .push_bind(pattern.clone())
            .push(" OR lower(consent_records.consent_type) LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(category_id) = filters.category_id {
        query
            .push(" AND consent_records.category_id = ")
            .push_bind(category_id);
    }
    if let Some(status) = filters.status {
        query
            .push(" AND consent_records.status = ")
            .push_bind(validate_status(&status)?.to_string());
    }
    if let Some(risk_level) = filters.risk_level {
        query
            .push(" AND consent_records.risk_level = ")
            .push_bind(validate_risk(&risk_level)?.to_string());
    }
    if let Some(source) = filters.source {
        query
            .push(" AND consent_records.source = ")
            .push_bind(validate_source(&source)?.to_string());
    }
    if filters.review_due.unwrap_or(false) {
        query.push(" AND (consent_records.review_date IS NOT NULL AND consent_records.review_date <= current_date OR consent_records.status = 'needs_review')");
    }
    if filters.expired.unwrap_or(false) {
        query.push(" AND (consent_records.expiry_date IS NOT NULL AND consent_records.expiry_date <= current_date OR consent_records.status = 'expired')");
    }

    match filters.sort.as_deref() {
        Some("risk") => query.push(" ORDER BY CASE consent_records.risk_level WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, consent_records.created_at DESC"),
        Some("status") => query.push(" ORDER BY consent_records.status, consent_records.created_at DESC"),
        Some("date_given") => query.push(" ORDER BY consent_records.date_given DESC, consent_records.created_at DESC"),
        _ => query.push(" ORDER BY consent_records.created_at DESC"),
    };
    query.push(" LIMIT ").push_bind(limit);

    Ok(query
        .build_query_as::<RecordResponse>()
        .fetch_all(pool)
        .await?)
}

pub async fn fetch_record(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> ApiResult<Option<RecordResponse>> {
    let sql = format!(
        "{} WHERE consent_records.user_id = $1 AND consent_records.id = $2",
        record_select_sql()
    );
    Ok(sqlx::query_as::<_, RecordResponse>(&sql)
        .bind(user_id)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

async fn ensure_record_owner(pool: &PgPool, user_id: Uuid, id: Uuid) -> ApiResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM consent_records WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

async fn validate_category(pool: &PgPool, category_id: Uuid) -> ApiResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM consent_categories WHERE id = $1)",
    )
    .bind(category_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::BadRequest("category does not exist".to_string()))
    }
}

/// Ensure the shared service row for a domain exists and return its id. The
/// touch of updated_at lets the statement return the id on conflict without
/// overwriting any user-facing data.
async fn upsert_service(pool: &PgPool, normalized_domain: &str) -> ApiResult<Uuid> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO services (normalized_domain)
        VALUES ($1)
        ON CONFLICT (normalized_domain) DO UPDATE
        SET updated_at = now()
        RETURNING id
        "#,
    )
    .bind(normalized_domain)
    .fetch_one(pool)
    .await?)
}

fn record_select_sql() -> &'static str {
    r#"
    SELECT
        consent_records.id,
        services.id AS service_id,
        consent_records.service_name,
        services.normalized_domain,
        consent_records.website_url,
        consent_records.category_id,
        consent_categories.name AS category_name,
        consent_records.consent_type,
        consent_records.date_given,
        consent_records.review_date,
        consent_records.expiry_date,
        consent_records.status,
        consent_records.risk_level,
        consent_records.source,
        consent_records.notes,
        consent_records.revoked_at,
        consent_records.created_at,
        consent_records.updated_at
    FROM consent_records
    JOIN services ON services.id = consent_records.service_id
    JOIN consent_categories ON consent_categories.id = consent_records.category_id
    "#
}

fn normalize_service(website_url: &str) -> ApiResult<String> {
    let parsed = Url::parse(website_url)
        .map_err(|_| ApiError::BadRequest("valid website URL is required".to_string()))?;
    let Some(host) = parsed.host_str() else {
        return Err(ApiError::BadRequest(
            "website URL must include a host".to_string(),
        ));
    };
    Ok(host.trim_start_matches("www.").to_lowercase())
}

fn validate_status(status: &str) -> ApiResult<&str> {
    match status {
        "active" | "needs_review" | "expired" | "revoked" => Ok(status),
        _ => Err(ApiError::BadRequest("invalid status".to_string())),
    }
}

fn validate_risk(risk_level: &str) -> ApiResult<&str> {
    match risk_level {
        "low" | "medium" | "high" => Ok(risk_level),
        _ => Err(ApiError::BadRequest("invalid risk level".to_string())),
    }
}

fn validate_source(source: &str) -> ApiResult<&str> {
    match source {
        "manual" | "extension" => Ok(source),
        _ => Err(ApiError::BadRequest("invalid source".to_string())),
    }
}

fn clean_required<'a>(value: &'a str, field: &str, max_len: usize) -> ApiResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }
    Ok(trimmed.chars().take(max_len).collect())
}

fn clean_optional(value: Option<String>, max_len: usize) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(max_len).collect())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_service_uses_domain_without_www() {
        assert_eq!(
            normalize_service("https://www.example.com/path").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn normalize_service_rejects_invalid_urls() {
        assert!(normalize_service("not a url").is_err());
    }

    #[test]
    fn consent_enums_accept_release_one_values() {
        assert_eq!(validate_status("needs_review").unwrap(), "needs_review");
        assert_eq!(validate_risk("high").unwrap(), "high");
        assert_eq!(validate_source("extension").unwrap(), "extension");
    }

    #[test]
    fn consent_enums_reject_unknown_values() {
        assert!(validate_status("archived").is_err());
        assert!(validate_risk("critical").is_err());
        assert!(validate_source("import").is_err());
    }
}
