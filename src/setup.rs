use argon2::{
    Argon2,
    password_hash::PasswordHasher,
};
use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use sqlx::PgPool;

/// First-run wizard, shown only while no accounts exist.
#[derive(Template)]
#[template(path = "setup.html")]
pub struct SetupTemplate;

/// Returns true when the wizard must be shown, i.e. no accounts exist yet.
pub async fn setup_required(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM accounts LIMIT 1)")
        .fetch_one(pool)
        .await?;
    Ok(!exists)
}

pub async fn get_setup(State(pool): State<PgPool>) -> Response {
    if !setup_required(&pool).await.unwrap_or(false) {
        return Redirect::to("/").into_response();
    }

    match SetupTemplate.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Deserialize)]
pub struct SetupForm {
    pub username: String,
    pub password: String,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
}

pub async fn post_setup(State(pool): State<PgPool>, _form: Form<SetupForm>) -> Response {
    if !setup_required(&pool).await.unwrap_or(false) {
        return Redirect::to("/").into_response();
    }

    // TODO: validate the submitted fields (non-empty, password strength, ...)
    // TODO: hash_password(&form.password), then insert the user and account
    //       rows in a single transaction
    Redirect::to("/").into_response()
}

/// Hash a plaintext password with Argon2id (same parameters as `hash_password` bin).
pub fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("Failed to hash password")
        .to_string()
}
