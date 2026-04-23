use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::Duration;

use crate::auth::{
    CurrentAccount, SESSION_COOKIE, SESSION_DURATION_DAYS, create_session, delete_session,
    verify_password,
};

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

#[derive(Serialize)]
pub(crate) struct AccountBody {
    account_id: i32,
    user_id: i32,
    username: String,
}

// Returns the currently-logged-in account, or 401.

pub async fn me(CurrentAccount(account): CurrentAccount) -> Json<AccountBody> {
    Json(AccountBody {
        account_id: account.account_id,
        user_id: account.user_id,
        username: account.username,
    })
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Response {
    // 1. Look up account by username.
    let row = sqlx::query!(
        "SELECT account_id, password_hash FROM accounts WHERE username = $1",
        body.username,
    )
    .fetch_optional(&pool)
    .await;

    let (authed, account_id) = match row {
        Ok(Some(r)) => (
            verify_password(&body.password, &r.password_hash),
            r.account_id,
        ),
        Ok(None) => {
            // Dummy verify to prevent timing-based username enumeration.
            let _ = verify_password(
                &body.password,
                "$argon2id$v=19$m=19456,t=2,p=1$aaaaaaaaaaaaaaaaaaaaaa$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            );
            (false, 0)
        }
        Err(e) => {
            tracing::error!("DB error during login: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "Internal server error",
                }),
            )
                .into_response();
        }
    };

    if !authed {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: "Invalid username or password",
            }),
        )
            .into_response();
    }

    // 2. Create session and set cookie.
    match create_session(&pool, account_id).await {
        Ok(token) => {
            let cookie = Cookie::build((SESSION_COOKIE, token))
                .http_only(true)
                .same_site(SameSite::Strict)
                .max_age(Duration::days(SESSION_DURATION_DAYS))
                .path("/")
                .build();

            (
                StatusCode::OK,
                jar.add(cookie),
                Json(serde_json::json!({ "ok": true })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create session: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "Internal server error",
                }),
            )
                .into_response()
        }
    }
}

pub async fn logout(State(pool): State<PgPool>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let _ = delete_session(&pool, cookie.value()).await;
    }

    let removal = Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(Duration::seconds(0))
        .path("/")
        .build();

    (jar.remove(removal), Json(serde_json::json!({ "ok": true }))).into_response()
}
