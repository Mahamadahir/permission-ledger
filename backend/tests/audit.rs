mod common;

use common::{authed, record_body};
use sqlx::PgPool;

async fn event_count(pool: &PgPool, event_type: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE event_type = $1")
        .bind(event_type)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn key_actions_are_recorded_in_the_audit_log() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "audit@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    common::login(&ctx.client(), "audit@example.com", "correcthorse1").await;

    let created = common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let mut update = record_body("Acme", "https://acme.example", &category);
    update["risk_level"] = serde_json::json!("high");
    authed(server.put(&format!("/api/records/{id}")), &auth)
        .json(&update)
        .await
        .assert_status_ok();
    authed(server.post(&format!("/api/records/{id}/revoke")), &auth)
        .await
        .assert_status_ok();
    authed(server.delete(&format!("/api/records/{id}")), &auth)
        .await
        .assert_status_ok();

    authed(server.post("/api/extension/pair"), &auth)
        .json(&serde_json::json!({ "device_name": "d" }))
        .await
        .assert_status_ok();

    for event in [
        "user.registered",
        "user.login",
        "consent.created",
        "consent.updated",
        "consent.revoked",
        "consent.deleted",
        "extension.paired",
    ] {
        assert!(event_count(&ctx.pool, event).await >= 1, "missing {event}");
    }
}

#[tokio::test]
async fn audit_log_never_stores_the_session_token() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "secret@example.com", "correcthorse1").await;

    // The raw session token from the cookie header must appear nowhere in the log.
    let token = auth
        .cookie_header
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("pl_session=");

    let leaked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE metadata::text LIKE '%' || $1 || '%'",
    )
    .bind(token)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(leaked, 0);
}
