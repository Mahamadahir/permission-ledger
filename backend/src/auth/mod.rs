use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    audit,
    error::{ApiError, ApiResult},
    AppState,
};

const SESSION_COOKIE: &str = "pl_session";
const CSRF_COOKIE: &str = "pl_csrf";
const SESSION_DAYS: i64 = 14;
const MAX_FAILED_LOGINS: i32 = 5;
const LOCKOUT_MINUTES: i64 = 15;

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserWithPassword {
    id: Uuid,
    email: String,
    password_hash: String,
    display_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct SessionUser {
    user_id: Uuid,
    csrf_token: String,
    email: String,
    display_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    display_name: Option<String>,
    timezone: Option<String>,
    review_reminder_days: Option<i32>,
}

#[derive(Serialize)]
struct AuthResponse {
    user: User,
    csrf_token: String,
}

#[derive(Serialize)]
struct SettingsResponse {
    display_name: Option<String>,
    timezone: String,
    review_reminder_days: i32,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/settings", get(get_settings).put(update_settings))
}

pub async fn require_user(pool: &PgPool, headers: &HeaderMap) -> ApiResult<(User, String)> {
    let token = read_cookie(headers, SESSION_COOKIE).ok_or(ApiError::Unauthorized)?;
    let token_hash = hash_secret(&token);
    let row = sqlx::query_as::<_, SessionUser>(
        r#"
        SELECT sessions.user_id, sessions.csrf_token, users.email, users.display_name, users.created_at
        FROM sessions
        JOIN users ON users.id = sessions.user_id
        WHERE sessions.token_hash = $1 AND sessions.expires_at > now()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    Ok((
        User {
            id: row.user_id,
            email: row.email,
            display_name: row.display_name,
            created_at: row.created_at,
        },
        row.csrf_token,
    ))
}

pub async fn require_csrf(pool: &PgPool, headers: &HeaderMap) -> ApiResult<User> {
    let (user, csrf_token) = require_user(pool, headers).await?;
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Forbidden)?;
    if supplied != csrf_token {
        return Err(ApiError::Forbidden);
    }
    Ok(user)
}

pub fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)
}

pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    let email = normalize_email(&payload.email)?;
    validate_password(&payload.password)?;
    let password_hash = hash_password(&payload.password)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, display_name)
        VALUES ($1, $2, $3)
        RETURNING id, email, display_name, created_at
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(clean_optional(payload.display_name, 120))
    .fetch_one(&state.pool)
    .await
    .map_err(map_unique_email)?;

    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    audit::log_event(
        &state.pool,
        Some(user.id),
        "user.registered",
        Some("user"),
        Some(user.id),
        serde_json::json!({}),
    )
    .await?;
    let (session_cookie, csrf_cookie, csrf_token) =
        create_session(&state, &headers, user.id).await?;
    let response = AuthResponse { user, csrf_token };
    Ok((
        StatusCode::CREATED,
        AppendHeaders([
            (header::SET_COOKIE, session_cookie),
            (header::SET_COOKIE, csrf_cookie),
        ]),
        Json(response),
    ))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let email = normalize_email(&payload.email)?;
    ensure_not_locked(&state.pool, &email).await?;
    let user = sqlx::query_as::<_, UserWithPassword>(
        "SELECT id, email, password_hash, display_name, created_at FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;

    let Some(user) = user else {
        record_failed_login(&state.pool, &email).await?;
        return Err(ApiError::Unauthorized);
    };

    if !verify_password(&payload.password, &user.password_hash)? {
        record_failed_login(&state.pool, &email).await?;
        return Err(ApiError::Unauthorized);
    }

    clear_failed_login(&state.pool, &email).await?;
    audit::log_event(
        &state.pool,
        Some(user.id),
        "user.login",
        Some("user"),
        Some(user.id),
        serde_json::json!({}),
    )
    .await?;
    let (session_cookie, csrf_cookie, csrf_token) =
        create_session(&state, &headers, user.id).await?;
    let response = AuthResponse {
        user: User {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            created_at: user.created_at,
        },
        csrf_token,
    };
    Ok((
        AppendHeaders([
            (header::SET_COOKIE, session_cookie),
            (header::SET_COOKIE, csrf_cookie),
        ]),
        Json(response),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    let user = require_csrf(&state.pool, &headers).await?;
    if let Some(token) = read_cookie(&headers, SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_secret(&token))
            .execute(&state.pool)
            .await?;
    }
    audit::log_event(
        &state.pool,
        Some(user.id),
        "user.logout",
        Some("user"),
        Some(user.id),
        serde_json::json!({}),
    )
    .await?;
    Ok((
        AppendHeaders([
            (header::SET_COOKIE, expired_cookie(SESSION_COOKIE)),
            (header::SET_COOKIE, expired_cookie(CSRF_COOKIE)),
        ]),
        StatusCode::NO_CONTENT,
    ))
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<AuthResponse>> {
    let (user, csrf_token) = require_user(&state.pool, &headers).await?;
    Ok(Json(AuthResponse { user, csrf_token }))
}

async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<SettingsResponse>> {
    let (user, _) = require_user(&state.pool, &headers).await?;
    let row = sqlx::query_as::<_, (Option<String>, String, i32)>(
        r#"
        SELECT users.display_name, user_settings.timezone, user_settings.review_reminder_days
        FROM users
        JOIN user_settings ON user_settings.user_id = users.id
        WHERE users.id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(SettingsResponse {
        display_name: row.0,
        timezone: row.1,
        review_reminder_days: row.2,
    }))
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateSettingsRequest>,
) -> ApiResult<Json<SettingsResponse>> {
    let user = require_csrf(&state.pool, &headers).await?;
    let timezone = clean_optional(payload.timezone, 64).unwrap_or_else(|| "UTC".to_string());
    let review_reminder_days = payload.review_reminder_days.unwrap_or(30).clamp(1, 365);
    let display_name = clean_optional(payload.display_name, 120);

    sqlx::query("UPDATE users SET display_name = $1, updated_at = now() WHERE id = $2")
        .bind(&display_name)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    sqlx::query("UPDATE user_settings SET timezone = $1, review_reminder_days = $2, updated_at = now() WHERE user_id = $3")
        .bind(&timezone)
        .bind(review_reminder_days)
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(SettingsResponse {
        display_name,
        timezone,
        review_reminder_days,
    }))
}

async fn create_session(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
) -> ApiResult<(String, String, String)> {
    let token = random_token();
    let csrf_token = random_token();
    let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(512).collect::<String>());

    sqlx::query(
        r#"
        INSERT INTO sessions (user_id, token_hash, csrf_token, user_agent, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(hash_secret(&token))
    .bind(&csrf_token)
    .bind(&user_agent)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    Ok((
        session_cookie(
            &state.config,
            SESSION_COOKIE,
            &token,
            true,
            Some(expires_at),
        ),
        session_cookie(
            &state.config,
            CSRF_COOKIE,
            &csrf_token,
            false,
            Some(expires_at),
        ),
        csrf_token,
    ))
}

fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal)
}

fn verify_password(password: &str, password_hash: &str) -> ApiResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| ApiError::Internal)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn normalize_email(email: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    if email.len() > 254 || !email.contains('@') {
        return Err(ApiError::BadRequest("valid email is required".to_string()));
    }
    Ok(email)
}

fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < 12 {
        return Err(ApiError::BadRequest(
            "password must be at least 12 characters".to_string(),
        ));
    }
    Ok(())
}

fn clean_optional(value: Option<String>, max_len: usize) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(max_len).collect())
        }
    })
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(cookie_name, value)| (cookie_name == name).then(|| value.to_string()))
}

fn session_cookie(
    config: &crate::config::Config,
    name: &str,
    value: &str,
    http_only: bool,
    expires_at: Option<DateTime<Utc>>,
) -> String {
    let mut cookie = format!("{name}={value}; Path=/; SameSite=Lax");
    if http_only {
        cookie.push_str("; HttpOnly");
    }
    if config.cookie_secure {
        cookie.push_str("; Secure");
    }
    if let Some(expires_at) = expires_at {
        cookie.push_str(&format!(
            "; Max-Age={}",
            (expires_at - Utc::now()).num_seconds()
        ));
    }
    cookie
}

fn expired_cookie(name: &str) -> String {
    format!("{name}=; Path=/; Max-Age=0; SameSite=Lax; HttpOnly")
}

async fn ensure_not_locked(pool: &PgPool, email: &str) -> ApiResult<()> {
    let locked_until = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT locked_until FROM login_rate_limits WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    .flatten();

    if locked_until.is_some_and(|until| until > Utc::now()) {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

async fn record_failed_login(pool: &PgPool, email: &str) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO login_rate_limits (email, failed_attempts, locked_until, updated_at)
        VALUES ($1, 1, NULL, now())
        ON CONFLICT (email) DO UPDATE
        SET failed_attempts = login_rate_limits.failed_attempts + 1,
            locked_until = CASE
                WHEN login_rate_limits.failed_attempts + 1 >= $2 THEN now() + ($3::text || ' minutes')::interval
                ELSE NULL
            END,
            updated_at = now()
        "#,
    )
    .bind(email)
    .bind(MAX_FAILED_LOGINS)
    .bind(LOCKOUT_MINUTES)
    .execute(pool)
    .await?;
    Ok(())
}

async fn clear_failed_login(pool: &PgPool, email: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM login_rate_limits WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await?;
    Ok(())
}

fn map_unique_email(error: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.constraint() == Some("users_email_key") {
            return ApiError::Conflict("email is already registered".to_string());
        }
    }
    ApiError::Database(error)
}
