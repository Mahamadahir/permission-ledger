use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::{
    audit, auth,
    error::ApiResult,
    records::{self, RecordFilters, RecordResponse},
    AppState,
};

#[derive(Serialize)]
struct JsonExport {
    format: &'static str,
    records: Vec<RecordResponse>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/export.json", get(export_json))
        .route("/export.csv", get(export_csv))
}

async fn export_json(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<JsonExport>> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let records = export_records(&state, user.id, "json").await?;
    Ok(Json(JsonExport {
        format: "json",
        records,
    }))
}

async fn export_csv(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let (user, _) = auth::require_user(&state.pool, &headers).await?;
    let records = export_records(&state, user.id, "csv").await?;
    let mut csv = String::from("id,service_name,website_url,category,consent_type,date_given,review_date,expiry_date,status,risk_level,source,notes,created_at,updated_at\n");
    for record in records {
        csv.push_str(
            &[
                record.id.to_string(),
                record.service_name,
                record.website_url,
                record.category_name,
                record.consent_type,
                record.date_given.to_string(),
                record
                    .review_date
                    .map(|date| date.to_string())
                    .unwrap_or_default(),
                record
                    .expiry_date
                    .map(|date| date.to_string())
                    .unwrap_or_default(),
                record.status,
                record.risk_level,
                record.source,
                record.notes.unwrap_or_default(),
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ]
            .into_iter()
            .map(csv_escape)
            .collect::<Vec<_>>()
            .join(","),
        );
        csv.push('\n');
    }

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=permission-ledger-export.csv",
            ),
        ],
        csv,
    )
        .into_response())
}

async fn export_records(
    state: &AppState,
    user_id: uuid::Uuid,
    format: &str,
) -> ApiResult<Vec<RecordResponse>> {
    sqlx::query("INSERT INTO export_jobs (user_id, format) VALUES ($1, $2)")
        .bind(user_id)
        .bind(format)
        .execute(&state.pool)
        .await?;
    audit::log_event(
        &state.pool,
        Some(user_id),
        "export.created",
        Some("export_job"),
        None,
        serde_json::json!({ "format": format }),
    )
    .await?;
    records::fetch_records(
        &state.pool,
        user_id,
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
        10_000,
    )
    .await
}

fn csv_escape(value: String) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}
