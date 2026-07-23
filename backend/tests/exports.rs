mod common;

use common::{cookie_only, record_body};

#[tokio::test]
async fn json_export_is_scoped_and_logs_a_job() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "json@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await;

    let export = cookie_only(server.get("/api/export.json"), &auth).await;
    export.assert_status_ok();
    let body = export.json::<serde_json::Value>();
    assert_eq!(body["format"], "json");
    assert_eq!(body["records"].as_array().unwrap().len(), 1);

    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM export_jobs WHERE format = 'json'")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE event_type = 'export.created'")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert_eq!(events, 1);
}

#[tokio::test]
async fn csv_export_has_header_and_one_row_per_record() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "csv@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await;

    let export = cookie_only(server.get("/api/export.csv"), &auth).await;
    export.assert_status_ok();
    let text = export.text();
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap(),
        "id,service_name,website_url,category,consent_type,date_given,review_date,expiry_date,status,risk_level,source,notes,created_at,updated_at"
    );
    assert_eq!(lines.filter(|l| !l.is_empty()).count(), 1);
}

#[tokio::test]
async fn csv_escapes_commas_quotes_and_newlines() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "escape@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let mut body = record_body("Acme", "https://acme.example", &category);
    body["notes"] = serde_json::json!("has a comma, a \"quote\" and a\nnewline");
    common::create_record(&server, &auth, &body).await;

    let text = cookie_only(server.get("/api/export.csv"), &auth)
        .await
        .text();
    // The notes field must be wrapped in quotes with the inner quote doubled.
    assert!(text.contains("\"has a comma, a \"\"quote\"\" and a\nnewline\""));
}
