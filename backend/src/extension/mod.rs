use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    audit, auth,
    error::{ApiError, ApiResult},
    records::{self, RecordRequest, RecordResponse},
    AppState,
};

#[derive(Deserialize)]
struct PairRequest {
    device_name: String,
}

#[derive(Serialize)]
struct PairResponse {
    id: Uuid,
    name: String,
    token: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
struct DeviceResponse {
    id: Uuid,
    name: String,
    last_used_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct DeviceAuth {
    id: Uuid,
    user_id: Uuid,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/extension/pair", post(pair_device))
        .route("/extension/devices", get(list_devices))
        .route("/extension/devices/:id", delete(revoke_device))
        .route("/extension/records", post(create_extension_record))
}

async fn pair_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PairRequest>,
) -> ApiResult<Json<PairResponse>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    let token = auth::random_token();
    let token_hash = auth::hash_secret(&token);
    let name = clean_device_name(&payload.device_name);
    let row = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        r#"
        INSERT INTO extension_devices (user_id, name, token_hash)
        VALUES ($1, $2, $3)
        RETURNING id, created_at
        "#,
    )
    .bind(user.id)
    .bind(&name)
    .bind(token_hash)
    .fetch_one(&state.pool)
    .await?;

    audit::log_event(
        &state.pool,
        Some(user.id),
        "extension.paired",
        Some("extension_device"),
        Some(row.0),
        serde_json::json!({ "device_name": name }),
    )
    .await?;
    Ok(Json(PairResponse {
        id: row.0,
        name,
        token,
        created_at: row.1,
    }))
}

async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<DeviceResponse>>> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let rows = sqlx::query_as::<_, DeviceResponse>(
        r#"
        SELECT id, name, last_used_at, revoked_at, created_at
        FROM extension_devices
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let user = auth::require_csrf(&state.pool, &headers).await?;
    let result = sqlx::query("UPDATE extension_devices SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    audit::log_event(
        &state.pool,
        Some(user.id),
        "extension.revoked",
        Some("extension_device"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(serde_json::json!({ "revoked": true })))
}

async fn create_extension_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecordRequest>,
) -> ApiResult<Json<RecordResponse>> {
    let device = require_extension_device(&state, &headers).await?;
    let record =
        records::create_record_for_user(&state.pool, device.user_id, payload, "extension").await?;
    sqlx::query("UPDATE extension_devices SET last_used_at = now() WHERE id = $1")
        .bind(device.id)
        .execute(&state.pool)
        .await?;
    audit::log_event(
        &state.pool,
        Some(device.user_id),
        "consent.created",
        Some("consent_record"),
        Some(record.id),
        serde_json::json!({ "source": "extension", "device_id": device.id }),
    )
    .await?;
    Ok(Json(record))
}

async fn require_extension_device(state: &AppState, headers: &HeaderMap) -> ApiResult<DeviceAuth> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;
    sqlx::query_as::<_, DeviceAuth>(
        r#"
        SELECT id, user_id
        FROM extension_devices
        WHERE token_hash = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(auth::hash_secret(token))
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)
}

fn clean_device_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Chrome extension".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}
