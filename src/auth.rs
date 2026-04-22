use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::CookieJar;
use rand::Rng;
use sqlx::PgPool;
use time::OffsetDateTime;

pub const SESSION_COOKIE: &str = "session";
pub const SESSION_DURATION_DAYS: i64 = 7;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: i32,
    pub user_id: i32,
    pub username: String,
}

/// Axum extractor: resolves the logged-in account from the session cookie.
/// Returns 303 → /login if no valid session exists.
pub struct CurrentAccount(pub Account);

// ── Password helpers ──────────────────────────────────────────────────────────

/// Verify a plaintext password against a stored argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// ── Session helpers ───────────────────────────────────────────────────────────

/// Generate a cryptographically-random 32-byte hex session token.
pub fn generate_session_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().r#gen();
    hex::encode(bytes)
}

/// Create a session row and return its token.
pub async fn create_session(pool: &PgPool, account_id: i32) -> Result<String, sqlx::Error> {
    let token = generate_session_token();
    let expires_at = OffsetDateTime::now_utc() + time::Duration::days(SESSION_DURATION_DAYS);

    sqlx::query!(
        "INSERT INTO sessions (token, account_id, expires_at) VALUES ($1, $2, $3)",
        token,
        account_id,
        expires_at,
    )
    .execute(pool)
    .await?;

    Ok(token)
}

/// Look up a session token and return the associated account (if valid & not expired).
pub async fn get_account_by_session(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Account>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT a.account_id, a.user_id, a.username
        FROM sessions s
        JOIN accounts a ON a.account_id = s.account_id
        WHERE s.token = $1
          AND s.expires_at > NOW()
        "#,
        token,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Account {
        account_id: r.account_id,
        user_id: r.user_id,
        username: r.username,
    }))
}

/// Delete a session by token (logout).
pub async fn delete_session(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE token = $1", token)
        .execute(pool)
        .await?;
    Ok(())
}

impl<S> FromRequestParts<S> for CurrentAccount
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|e| e.into_response())?;

        let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned());

        match token {
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Not authenticated" })),
            )
                .into_response()),
            Some(token) => match get_account_by_session(&pool, &token).await {
                Ok(Some(account)) => Ok(CurrentAccount(account)),
                Ok(None) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Session expired" })),
                )
                    .into_response()),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
            },
        }
    }
}
