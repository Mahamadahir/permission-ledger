mod common;

use axum_test::TestServer;
use http::StatusCode;
use sqlx::PgPool;

async fn attempt_login(server: &TestServer, email: &str, password: &str) -> StatusCode {
    server
        .post("/api/auth/login")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .await
        .status_code()
}

async fn lock_state(pool: &PgPool, email: &str) -> (i32, bool) {
    let row: Option<(i32, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT failed_attempts, locked_until FROM login_rate_limits WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .unwrap();
    match row {
        Some((attempts, locked)) => (attempts, locked.is_some()),
        None => (0, false),
    }
}

#[tokio::test]
async fn five_failures_lock_out_even_with_correct_password() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "lock@example.com", "correcthorse1").await;

    for _ in 0..5 {
        let status = attempt_login(&ctx.client(), "lock@example.com", "wrongpassword1").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    let correct = attempt_login(&ctx.client(), "lock@example.com", "correcthorse1").await;
    assert_eq!(
        correct,
        StatusCode::FORBIDDEN,
        "correct password should be locked out"
    );
}

#[tokio::test]
async fn the_fifth_failure_is_the_one_that_locks() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "boundary@example.com", "correcthorse1").await;

    for _ in 0..4 {
        attempt_login(&ctx.client(), "boundary@example.com", "wrongpassword1").await;
    }
    let (attempts, locked) = lock_state(&ctx.pool, "boundary@example.com").await;
    assert_eq!(attempts, 4);
    assert!(!locked, "not locked after four failures");

    attempt_login(&ctx.client(), "boundary@example.com", "wrongpassword1").await;
    let (attempts, locked) = lock_state(&ctx.pool, "boundary@example.com").await;
    assert_eq!(attempts, 5);
    assert!(locked, "locked after the fifth failure");
}

#[tokio::test]
async fn a_successful_login_clears_the_counter() {
    let ctx = common::setup().await;
    common::register(&ctx.client(), "clear@example.com", "correcthorse1").await;

    for _ in 0..3 {
        attempt_login(&ctx.client(), "clear@example.com", "wrongpassword1").await;
    }
    assert_eq!(lock_state(&ctx.pool, "clear@example.com").await.0, 3);

    let ok = attempt_login(&ctx.client(), "clear@example.com", "correcthorse1").await;
    assert_eq!(ok, StatusCode::OK);

    let row: i64 = sqlx::query_scalar("SELECT count(*) FROM login_rate_limits WHERE email = $1")
        .bind("clear@example.com")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(row, 0, "rate-limit row deleted on success");
}

#[tokio::test]
async fn one_failure_after_lockout_expiry_re_locks_immediately() {
    // Documents that failed_attempts is never reset except on success, so a
    // single failure after the lock window re-locks. If this is later fixed to
    // reset the counter on expiry, this test should be updated.
    let ctx = common::setup().await;
    common::register(&ctx.client(), "relock@example.com", "correcthorse1").await;

    for _ in 0..5 {
        attempt_login(&ctx.client(), "relock@example.com", "wrongpassword1").await;
    }
    // Simulate the lock window having passed.
    sqlx::query(
        "UPDATE login_rate_limits SET locked_until = now() - interval '1 minute' WHERE email = $1",
    )
    .bind("relock@example.com")
    .execute(&ctx.pool)
    .await
    .unwrap();

    // A single failure re-locks because failed_attempts is still 5.
    attempt_login(&ctx.client(), "relock@example.com", "wrongpassword1").await;
    let correct = attempt_login(&ctx.client(), "relock@example.com", "correcthorse1").await;
    assert_eq!(correct, StatusCode::FORBIDDEN);
}
