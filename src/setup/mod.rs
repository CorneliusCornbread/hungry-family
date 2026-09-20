use crate::setup::template::{SetupForm, SetupPage};
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

pub mod template;


/// Returns true when the wizard must be shown, i.e. no accounts exist yet.
pub async fn setup_required(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM accounts)")
        .fetch_one(pool)
        .await?;
    Ok(!exists)
}

pub async fn get_setup(State(pool): State<PgPool>) -> Response {
    if !setup_required(&pool).await.unwrap_or(false) {
        return Redirect::to("/").into_response();
    }

    let setup = SetupPage {
        form: &SetupForm::default(),
        error: None,
    }.render();

    match setup {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}


pub async fn post_setup(State(pool): State<PgPool>, form: Form<SetupForm>) -> Response {
    if !setup_required(&pool).await.unwrap_or(false) {
        return Redirect::to("/").into_response();
    }

    // TODO: validate the submitted fields (non-empty, password strength, ...)
    // TODO: hash_password(&form.password), then insert the user and account
    //       rows in a single transaction

    if !is_valid_email(form.email.as_str()) {
        todo!()
        //return (StatusCode::UNPROCESSABLE_ENTITY).into_response();
    }

    // Suggest stronger password if the password is too weak
    const PASSWORD_STRENGTH_THRESHOLD: usize = 2;
    if password_strength(form.password.as_str()) < PASSWORD_STRENGTH_THRESHOLD {
        todo!()
    }

    let hash_pass = hash_password(&form.password);

    Redirect::to("/").into_response()
}

fn is_valid_email(email: &str) -> bool {
    let mut tld_split = email.split("@");

    if tld_split.clone().count() != 2 {
        return false;
    }

    let username = tld_split.next().unwrap();
    let tld = tld_split.next().unwrap();

    if username.contains("..") {
        return false;
    }

    // Is the username for the email (everything before the '@') valid
    // according to RFC 5322
    if !is_valid_email_user(username) {
        return false;
    }

    if !is_valid_tld(tld) {
        return false;
    }

    true
}

fn is_valid_email_user(username: &str) -> bool {
    let mut first_quote = false;
    let mut last_quote = false;

    let mut username_chars = username.chars();

    // Do the emails start or end with periods or quotes for quoted emails?
    if let Some(first_char) = username_chars.next() {
        if first_char == '.' {
            return false;
        } else if first_char == '\"' {
            first_quote = true;
        }
    }

    if let Some(last_char) = username_chars.last() {
        if last_char == '.' {
            return false;
        } else if last_char == '\"' {
            last_quote = true;
        }
    }

    // For some fuck ass reason email spec lets you quote emails.
    // While its rarely supported might as well fucking deal with it.
    let mut quoted_email = false;


    match (first_quote, last_quote) {
        (true, true) => { quoted_email = true }
        (true, false) | (false, true) => return false,
        _ => { () }
    };

    // Only quoted emails may contain spaces
    let validated_chars = if !quoted_email {
        if username.contains(" ") {
            return false;
        }

        username
    }
    // Quoted username validation
    else {
        let mut email_user_quote_split = username.split('\"');

        if email_user_quote_split.clone().count() != 2 {
            return false;
        }

        email_user_quote_split.next().unwrap()
    };

    // Return true if all chars are valid
    validated_chars.chars().all(|c| {
        match c {
            '0'..='9' |
            'A'..='Z' |
            'a'..='z' |
            '!' |
            '#' |
            '$' |
            '%' |
            '&' |
            '\'' |
            '*' |
            '+' |
            '-' |
            '/' |
            '=' |
            '?' |
            '^' |
            '_' |
            '`' |
            '{' |
            '|' |
            '}' |
            '~' => true,
            ' ' => {
                // Spaces are only valid if this is a quoted email
                if quoted_email {
                    return true;
                };

                false
            }
            _ => false
        }
    })
}

fn is_valid_tld(tld: &str) -> bool {
    if tld.len() > 255 {
        return false;
    }

    let tld_split = tld.split('.');

    for split in tld_split {
        // If there's any double periods they will result in an empty split
        if split.is_empty() {
            return false;
        }

        if split.len() > 63 {
            return false;
        }

        if split.starts_with('-') || split.ends_with('-') {
            return false;
        }

        let alpha_count = split.chars().filter(|c| c.is_alphabetic()).count();
        let valid = split.chars().all(|c| c.is_alphanumeric() || c == '-');

        if !valid && alpha_count > 0 {
            return false;
        }
    }

    true
}

fn password_strength(password: &str) -> usize {
    let mut strength = 0;

    match password.len() {
        8..12 => strength += 1,
        12..24 => strength += 2,
        24.. => strength += 3,
        _ => {}
    }

    // Add strength based on the number of special characters
    strength += password.chars().filter(|c| c.is_ascii_punctuation()).count();
    strength
}

/// Hash a plaintext password with Argon2id (same parameters as `hash_password` bin).
pub fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("Failed to hash password")
        .to_string()
}
