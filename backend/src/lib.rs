pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod exports;
pub mod extension;
pub mod records;

use axum::{
    http::{HeaderValue, Method, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn app(state: AppState) -> Router {
    let cors_origin = state
        .config
        .web_origin
        .parse::<HeaderValue>()
        .expect("WEB_ORIGIN must be a valid header value");
    Router::new()
        .route("/health", get(health))
        .nest("/api/auth", auth::router())
        .nest("/api", records::router())
        .nest("/api", extension::router())
        .nest("/api", exports::router())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(cors_origin))
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::HeaderName::from_static("x-csrf-token"),
                ])
                .allow_credentials(true),
        )
        .with_state(state)
}

async fn health() -> (StatusCode, Json<HealthResponse>) {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}
