use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

pub async fn log_event(
    pool: &PgPool,
    user_id: Option<Uuid>,
    event_type: &str,
    entity_type: Option<&str>,
    entity_id: Option<Uuid>,
    metadata: Value,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (user_id, event_type, entity_type, entity_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(event_type)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .execute(pool)
    .await?;

    Ok(())
}
