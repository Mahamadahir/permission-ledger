mod common;

use common::{authed, cookie_only};
use http::StatusCode;

#[tokio::test]
async fn register_sets_cookies_and_creates_settings() {
    let ctx = common::setup().await;
    let server = ctx.client();

    let response = server
        .post("/api/auth/register")
        .json(&serde_json::json!({ "email": "a@example.com", "password": "correcthorse1" }))
        .await;
    response.assert_status(StatusCode::CREATED);
    assert!(response.maybe_cookie("pl_session").is_some());
    assert!(response.maybe_cookie("pl_csrf").is_some());

    let settings_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM user_settings WHERE user_id = (SELECT id FROM users WHERE email = $1)",
    )
    .bind("a@example.com")
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(settings_rows, 1);
}

#[tokio::test]
async fn register_rejects_short_password() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let response = server
        .post("/api/auth/register")
        .json(&serde_json::json!({ "email": "a@example.com", "password": "short" }))
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_rejects_email_without_at_sign() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let response = server
        .post("/api/auth/register")
        .json(&serde_json::json!({ "email": "not-an-email", "password": "correcthorse1" }))
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn email_is_trimmed_and_lowercased() {
    let ctx = common::setup().await;
    let server = ctx.client();
    server
        .post("/api/auth/register")
        .json(
            &serde_json::json!({ "email": "  MixedCase@Example.COM ", "password": "correcthorse1" }),
        )
        .await
        .assert_status(StatusCode::CREATED);

    let stored: String = sqlx::query_scalar("SELECT email FROM users LIMIT 1")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(stored, "mixedcase@example.com");

    // Login with a differently-cased form of the same address succeeds.
    common::login(&ctx.client(), "MIXEDCASE@example.com", "correcthorse1").await;
}

#[tokio::test]
async fn duplicate_email_conflicts() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "dup@example.com", "correcthorse1").await;

    let response = ctx
        .client()
        .post("/api/auth/register")
        .json(&serde_json::json!({ "email": "dup@example.com", "password": "correcthorse1" }))
        .await;
    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn login_wrong_password_and_unknown_email_both_401() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "user@example.com", "correcthorse1").await;

    ctx.client()
        .post("/api/auth/login")
        .json(&serde_json::json!({ "email": "user@example.com", "password": "wrongpassword1" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    ctx.client()
        .post("/api/auth/login")
        .json(&serde_json::json!({ "email": "nobody@example.com", "password": "correcthorse1" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_requires_a_session() {
    let ctx = common::setup().await;
    ctx.client()
        .get("/api/auth/me")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_requires_csrf_then_clears_session() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "out@example.com", "correcthorse1").await;

    // Missing CSRF header is rejected and the session survives.
    cookie_only(server.post("/api/auth/logout"), &auth)
        .await
        .assert_status(StatusCode::FORBIDDEN);
    cookie_only(server.get("/api/auth/me"), &auth)
        .await
        .assert_status_ok();

    authed(server.post("/api/auth/logout"), &auth)
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    cookie_only(server.get("/api/auth/me"), &auth)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_session_is_rejected() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "expiry@example.com", "correcthorse1").await;

    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 hour'")
        .execute(&ctx.pool)
        .await
        .unwrap();

    cookie_only(server.get("/api/auth/me"), &auth)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn review_reminder_days_is_clamped() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "settings@example.com", "correcthorse1").await;

    let low = authed(server.put("/api/auth/settings"), &auth)
        .json(&serde_json::json!({ "review_reminder_days": 0 }))
        .await;
    low.assert_status_ok();
    assert_eq!(low.json::<serde_json::Value>()["review_reminder_days"], 1);

    let high = authed(server.put("/api/auth/settings"), &auth)
        .json(&serde_json::json!({ "review_reminder_days": 400 }))
        .await;
    high.assert_status_ok();
    assert_eq!(
        high.json::<serde_json::Value>()["review_reminder_days"],
        365
    );
}
