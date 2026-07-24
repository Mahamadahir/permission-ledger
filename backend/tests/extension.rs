mod common;

use common::{authed, record_body};
use http::StatusCode;

async fn pair(server: &axum_test::TestServer, auth: &common::Auth) -> (String, String) {
    let response = authed(server.post("/api/extension/pair"), auth)
        .json(&serde_json::json!({ "device_name": "Test device" }))
        .await;
    response.assert_status_ok();
    let body = response.json::<serde_json::Value>();
    (
        body["id"].as_str().unwrap().to_string(),
        body["token"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn pairing_returns_token_once_and_stores_only_a_hash() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "pair@example.com", "correcthorse1").await;
    let (_id, token) = pair(&server, &auth).await;

    let stored_hash: String =
        sqlx::query_scalar("SELECT token_hash FROM extension_devices LIMIT 1")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert_ne!(stored_hash, token, "raw token must not be stored");
    assert_eq!(stored_hash.len(), 64, "sha256 hex hash");
}

#[tokio::test]
async fn extension_token_creates_record_and_updates_last_used() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "ext@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    let (id, token) = pair(&server, &auth).await;

    let created = server
        .post("/api/extension/records")
        .authorization_bearer(&token)
        .json(&record_body(
            "Captured",
            "https://captured.example",
            &category,
        ))
        .await;
    created.assert_status_ok();
    assert_eq!(created.json::<serde_json::Value>()["source"], "extension");

    let last_used: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT last_used_at FROM extension_devices WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&id).unwrap())
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert!(last_used.is_some());
}

#[tokio::test]
async fn unknown_and_revoked_tokens_are_rejected() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "revoke@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    let (id, token) = pair(&server, &auth).await;

    server
        .post("/api/extension/records")
        .authorization_bearer("not-a-real-token")
        .json(&record_body("X", "https://x.example", &category))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    authed(
        server.delete(&format!("/api/extension/devices/{id}")),
        &auth,
    )
    .await
    .assert_status_ok();

    server
        .post("/api/extension/records")
        .authorization_bearer(&token)
        .json(&record_body("X", "https://x.example", &category))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoking_a_device_twice_is_not_found_the_second_time() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "twice@example.com", "correcthorse1").await;
    let (id, _token) = pair(&server, &auth).await;

    authed(
        server.delete(&format!("/api/extension/devices/{id}")),
        &auth,
    )
    .await
    .assert_status_ok();
    authed(
        server.delete(&format!("/api/extension/devices/{id}")),
        &auth,
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn revoked_devices_drop_out_of_the_device_list() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "listing@example.com", "correcthorse1").await;
    let (id, _token) = pair(&server, &auth).await;

    let listed = common::cookie_only(server.get("/api/extension/devices"), &auth).await;
    assert_eq!(listed.json::<Vec<serde_json::Value>>().len(), 1);

    authed(
        server.delete(&format!("/api/extension/devices/{id}")),
        &auth,
    )
    .await
    .assert_status_ok();

    let after = common::cookie_only(server.get("/api/extension/devices"), &auth).await;
    assert_eq!(
        after.json::<Vec<serde_json::Value>>().len(),
        0,
        "a revoked device is no longer paired"
    );
    // The device row still exists, it is just filtered from the paired list.
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM extension_devices")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn cookie_routes_reject_a_bearer_only_client() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "bearer@example.com", "correcthorse1").await;
    let (_id, token) = pair(&server, &auth).await;

    // A device token is not a session; it cannot list devices.
    server
        .get("/api/extension/devices")
        .authorization_bearer(&token)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}
