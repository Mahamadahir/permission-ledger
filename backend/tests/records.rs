mod common;

use common::{authed, cookie_only, record_body};
use http::StatusCode;

#[tokio::test]
async fn record_lifecycle_create_update_revoke_delete() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "life@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let created = common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await;
    assert_eq!(created["source"], "manual");
    assert_eq!(created["status"], "active");
    let id = created["id"].as_str().unwrap().to_string();

    cookie_only(server.get(&format!("/api/records/{id}")), &auth)
        .await
        .assert_status_ok();

    let mut update = record_body("Acme Renamed", "https://acme.example", &category);
    update["risk_level"] = serde_json::json!("high");
    let updated = authed(server.put(&format!("/api/records/{id}")), &auth)
        .json(&update)
        .await;
    updated.assert_status_ok();
    assert_eq!(updated.json::<serde_json::Value>()["risk_level"], "high");

    let revoked = authed(server.post(&format!("/api/records/{id}/revoke")), &auth).await;
    revoked.assert_status_ok();
    let revoked_body = revoked.json::<serde_json::Value>();
    assert_eq!(revoked_body["status"], "revoked");
    assert!(!revoked_body["revoked_at"].is_null());

    authed(server.delete(&format!("/api/records/{id}")), &auth)
        .await
        .assert_status_ok();
    cookie_only(server.get(&format!("/api/records/{id}")), &auth)
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_rejects_unknown_category() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "cat@example.com", "correcthorse1").await;
    let body = record_body(
        "Acme",
        "https://acme.example",
        &uuid::Uuid::new_v4().to_string(),
    );
    authed(server.post("/api/records"), &auth)
        .json(&body)
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_rejects_malformed_url() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "url@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    let body = record_body("Acme", "not-a-url", &category);
    authed(server.post("/api/records"), &auth)
        .json(&body)
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mutations_require_csrf() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "csrf@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    let created = common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    // Session present, CSRF header missing -> 403.
    cookie_only(server.post("/api/records"), &auth)
        .json(&record_body("Beta", "https://beta.example", &category))
        .await
        .assert_status(StatusCode::FORBIDDEN);
    cookie_only(server.post(&format!("/api/records/{id}/revoke")), &auth)
        .await
        .assert_status(StatusCode::FORBIDDEN);
    cookie_only(server.delete(&format!("/api/records/{id}")), &auth)
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_filters_by_status_risk_and_search() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "filter@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let mut high = record_body("HighRisk Bank", "https://bank.example", &category);
    high["risk_level"] = serde_json::json!("high");
    common::create_record(&server, &auth, &high).await;

    let low = record_body("Newsletter", "https://news.example", &category);
    common::create_record(&server, &auth, &low).await;

    let by_risk = cookie_only(server.get("/api/records?risk_level=high"), &auth).await;
    by_risk.assert_status_ok();
    let rows = by_risk.json::<Vec<serde_json::Value>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["service_name"], "HighRisk Bank");

    let by_search = cookie_only(server.get("/api/records?q=newsletter"), &auth).await;
    let rows = by_search.json::<Vec<serde_json::Value>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["service_name"], "Newsletter");

    // Invalid enum filter value is rejected rather than silently ignored.
    cookie_only(server.get("/api/records?status=archived"), &auth)
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn dashboard_summary_counts_each_bucket() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "dash@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    // Active + high risk.
    let mut high = record_body("Bank", "https://bank.example", &category);
    high["risk_level"] = serde_json::json!("high");
    common::create_record(&server, &auth, &high).await;

    // A record due for review (past review_date) and needs_review status.
    let mut review = record_body("Review", "https://review.example", &category);
    review["status"] = serde_json::json!("needs_review");
    review["review_date"] = serde_json::json!("2020-01-01");
    common::create_record(&server, &auth, &review).await;

    // A revoked record.
    let revoked = common::create_record(
        &server,
        &auth,
        &record_body("Gone", "https://gone.example", &category),
    )
    .await;
    authed(
        server.post(&format!(
            "/api/records/{}/revoke",
            revoked["id"].as_str().unwrap()
        )),
        &auth,
    )
    .await
    .assert_status_ok();

    let dashboard = cookie_only(server.get("/api/dashboard"), &auth).await;
    dashboard.assert_status_ok();
    let summary = dashboard.json::<serde_json::Value>()["summary"].clone();
    assert_eq!(summary["active"], 1); // only Bank; Review is needs_review, Gone is revoked
    assert_eq!(summary["high_risk"], 1);
    assert_eq!(summary["review_due"], 1);
    assert_eq!(summary["revoked"], 1);
}
