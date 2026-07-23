mod common;

use common::{authed, record_body};
use futures::future::join_all;

#[tokio::test]
async fn concurrent_records_for_one_domain_create_a_single_service() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "dedup@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    // A fresh server per task (all share the same pool and session).
    let servers: Vec<_> = (0..8).map(|_| ctx.client()).collect();
    let mut tasks = Vec::new();
    for (i, task_server) in servers.iter().enumerate() {
        let auth_cookie = auth.cookie_header.clone();
        let csrf = auth.csrf.clone();
        let body = record_body(&format!("Name {i}"), "https://same.example/path", &category);
        tasks.push(async move {
            task_server
                .post("/api/records")
                .add_header("cookie", auth_cookie)
                .add_header("x-csrf-token", csrf)
                .json(&body)
                .await
        });
    }
    let responses = join_all(tasks).await;
    for response in &responses {
        response.assert_status_ok();
    }

    let services: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM services WHERE normalized_domain = 'same.example'",
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(
        services, 1,
        "concurrent upserts must not duplicate the service"
    );
}

#[tokio::test]
async fn concurrent_failed_logins_do_not_lose_increments() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "concurrent@example.com", "correcthorse1").await;

    let mut tasks = Vec::new();
    for _ in 0..5 {
        let server = ctx.client();
        tasks.push(async move {
            server
                .post("/api/auth/login")
                .json(&serde_json::json!({ "email": "concurrent@example.com", "password": "wrongpassword1" }))
                .await
        });
    }
    join_all(tasks).await;

    let (attempts, locked): (i32, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT failed_attempts, locked_until FROM login_rate_limits WHERE email = $1",
    )
    .bind("concurrent@example.com")
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(attempts, 5, "no increments lost under concurrency");
    assert!(locked.is_some(), "account locked after five failures");
}

#[tokio::test]
async fn concurrent_updates_leave_a_consistent_row() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "update-race@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    let id = common::create_record(
        &server,
        &auth,
        &record_body("Acme", "https://acme.example", &category),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut first = record_body("Acme", "https://acme.example", &category);
    first["risk_level"] = serde_json::json!("high");
    let mut second = record_body("Acme", "https://acme.example", &category);
    second["risk_level"] = serde_json::json!("low");

    let (a, b) = tokio::join!(
        authed(server.put(&format!("/api/records/{id}")), &auth).json(&first),
        authed(server.put(&format!("/api/records/{id}")), &auth).json(&second),
    );
    a.assert_status_ok();
    b.assert_status_ok();

    // Exactly one row, with one of the two written risk levels (last-write-wins).
    let (count, risk): (i64, String) = sqlx::query_as(
        "SELECT count(*) OVER (), risk_level FROM consent_records WHERE id = $1 LIMIT 1",
    )
    .bind(uuid::Uuid::parse_str(&id).unwrap())
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!(risk == "high" || risk == "low");
}
