// Each test binary includes this module but uses a different subset of helpers.
#![allow(dead_code)]

use axum_test::{TestRequest, TestServer};
use permission_ledger_backend::{app, config::Config, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use std::net::SocketAddr;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

/// A per-test Postgres database plus the shared application state. Each test gets
/// its own database so global tables (login_rate_limits, services) never bleed
/// between tests, and the database is dropped when the context falls out of scope.
pub struct TestContext {
    pub pool: PgPool,
    state: AppState,
    maintenance_url: String,
    db_name: String,
}

impl TestContext {
    /// A fresh server with its own cookie jar, sharing the test database. Call
    /// once per simulated client so two users don't share a session cookie.
    pub fn client(&self) -> TestServer {
        // Auto cookie saving is deliberately left off: axum-test's mock-transport
        // jar intermittently drops the HttpOnly session cookie under parallel
        // runtimes. Tests carry session + CSRF explicitly via `Auth` instead.
        TestServer::new(app(self.state.clone())).expect("build test server")
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let maintenance_url = self.maintenance_url.clone();
        let db_name = self.db_name.clone();
        // Drop on a separate thread with its own runtime: the current test
        // runtime may be shutting down, and DROP DATABASE cannot run on a
        // connection to the database being dropped.
        let _ = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("drop runtime");
            runtime.block_on(async move {
                if let Ok(pool) = PgPool::connect(&maintenance_url).await {
                    let _ = pool
                        .execute(
                            format!("DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)").as_str(),
                        )
                        .await;
                }
            });
        })
        .join();
    }
}

fn base_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://permission_ledger:permission_ledger@localhost:5432/permission_ledger"
            .to_string()
    })
}

fn url_with_database(base: &str, database: &str) -> String {
    let mut url = Url::parse(base).expect("valid DATABASE_URL");
    url.set_path(database);
    url.to_string()
}

pub async fn setup() -> TestContext {
    let base = base_database_url();
    let maintenance_url = url_with_database(&base, "postgres");
    let db_name = format!("pl_test_{}", Uuid::new_v4().simple());

    let admin = PgPool::connect(&maintenance_url)
        .await
        .expect("connect to maintenance database");
    admin
        .execute(format!("CREATE DATABASE \"{db_name}\"").as_str())
        .await
        .expect("create test database");
    admin.close().await;

    let test_url = url_with_database(&base, &db_name);
    // Keep each test's pool small so many parallel tests stay well under the
    // server's connection limit.
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&test_url)
        .await
        .expect("connect to test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let config = Config {
        bind_addr: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
        database_url: test_url,
        cookie_secure: false,
        web_origin: "http://localhost:5173".to_string(),
    };

    let state = AppState {
        pool: pool.clone(),
        config,
    };

    TestContext {
        pool,
        state,
        maintenance_url,
        db_name,
    }
}

/// An authenticated identity. Carries the session and CSRF cookies as an
/// explicit Cookie header rather than relying on axum-test's cookie jar, which
/// drops the HttpOnly session cookie under parallel test runtimes.
pub struct Auth {
    pub cookie_header: String,
    pub csrf: String,
}

fn auth_from_response(response: &axum_test::TestResponse) -> Auth {
    let session = response.cookie("pl_session").value().to_string();
    let csrf = response.cookie("pl_csrf").value().to_string();
    Auth {
        cookie_header: format!("pl_session={session}; pl_csrf={csrf}"),
        csrf,
    }
}

/// Attach session cookie and CSRF header, for authenticated mutating requests.
pub fn authed(request: TestRequest, auth: &Auth) -> TestRequest {
    request
        .add_header("cookie", auth.cookie_header.clone())
        .add_header("x-csrf-token", auth.csrf.clone())
}

/// Attach only the session cookie, for read requests or to test missing CSRF.
pub fn cookie_only(request: TestRequest, auth: &Auth) -> TestRequest {
    request.add_header("cookie", auth.cookie_header.clone())
}

pub async fn register(server: &TestServer, email: &str, password: &str) -> Auth {
    let response = server
        .post("/api/auth/register")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .await;
    response.assert_status(http::StatusCode::CREATED);
    auth_from_response(&response)
}

pub async fn login(server: &TestServer, email: &str, password: &str) -> Auth {
    let response = server
        .post("/api/auth/login")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .await;
    response.assert_status_ok();
    auth_from_response(&response)
}

/// The id of the first seeded consent category, for building record payloads.
pub async fn first_category_id(server: &TestServer) -> String {
    let response = server.get("/api/categories").await;
    response.assert_status_ok();
    response.json::<serde_json::Value>()[0]["id"]
        .as_str()
        .expect("category id")
        .to_string()
}

/// A valid manual-record payload.
pub fn record_body(service_name: &str, website_url: &str, category_id: &str) -> serde_json::Value {
    serde_json::json!({
        "service_name": service_name,
        "website_url": website_url,
        "consent_type": "cookies",
        "category_id": category_id,
        "date_given": "2026-01-01",
        "risk_level": "low"
    })
}

/// Create a manual record and return the created record JSON.
pub async fn create_record(
    server: &TestServer,
    auth: &Auth,
    body: &serde_json::Value,
) -> serde_json::Value {
    let response = authed(server.post("/api/records"), auth).json(body).await;
    response.assert_status_ok();
    response.json::<serde_json::Value>()
}
