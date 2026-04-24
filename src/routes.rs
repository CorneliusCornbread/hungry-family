use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::Duration;
use tracing::info;

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

#[tracing::instrument(skip_all, fields(username = body.username))]
pub async fn login(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Response {
    info!("Login attempt");
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

#[derive(Serialize)]
pub struct StoreLayoutBody {
    layout_id: i32,
    label: String,
    sort_order: i32,
}

#[derive(Serialize)]
pub struct PlannerStoreBody {
    store_id: i32,
    name: String,
    address: String,
    layouts: Vec<StoreLayoutBody>,
}

#[derive(Serialize)]
pub struct PlannerProductBody {
    product_id: i32,
    name: String,
    store_id: i32,
    aisle_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateStoreRequest {
    name: String,
    address: String,
}

#[derive(Deserialize)]
pub struct CreateLayoutRequest {
    label: String,
}

#[derive(Deserialize)]
pub struct UpdateLayoutRequest {
    label: String,
    sort_order: i32,
}

#[derive(Deserialize)]
pub struct AssignProductRequest {
    product_id: i32,
    layout_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateProductRequest {
    name: String,
}

pub async fn planner_stores(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<PlannerStoreBody>>, StatusCode> {
    let store_rows = sqlx::query("SELECT store_id, name, address FROM stores ORDER BY name ASC")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let layout_rows = sqlx::query(
        "SELECT layout_id, store_id, label, sort_order FROM store_layouts ORDER BY store_id ASC, sort_order ASC, layout_id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut stores: Vec<PlannerStoreBody> = store_rows
        .into_iter()
        .map(|row| PlannerStoreBody {
            store_id: row.get("store_id"),
            name: row.get("name"),
            address: row.get("address"),
            layouts: Vec::new(),
        })
        .collect();

    for layout in layout_rows {
        let store_id: i32 = layout.get("store_id");
        if let Some(store) = stores.iter_mut().find(|s| s.store_id == store_id) {
            store.layouts.push(StoreLayoutBody {
                layout_id: layout.get("layout_id"),
                label: layout.get("label"),
                sort_order: layout.get("sort_order"),
            });
        }
    }

    Ok(Json(stores))
}

pub async fn create_planner_store(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Json(body): Json<CreateStoreRequest>,
) -> Result<Json<PlannerStoreBody>, StatusCode> {
    let name = body.name.trim();
    let address = body.address.trim();
    if name.is_empty() || address.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = sqlx::query(
        "INSERT INTO stores (name, address) VALUES ($1, $2) RETURNING store_id, name, address",
    )
    .bind(name)
    .bind(address)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PlannerStoreBody {
        store_id: row.get("store_id"),
        name: row.get("name"),
        address: row.get("address"),
        layouts: Vec::new(),
    }))
}

pub async fn create_store_layout(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<CreateLayoutRequest>,
) -> Result<Json<StoreLayoutBody>, StatusCode> {
    let label = body.label.trim();
    if label.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = sqlx::query(
        r#"
        INSERT INTO store_layouts (store_id, label, sort_order)
        VALUES (
          $1,
          $2,
          COALESCE((SELECT MAX(sort_order) + 1 FROM store_layouts WHERE store_id = $1), 1)
        )
        RETURNING layout_id, label, sort_order
        "#,
    )
    .bind(store_id)
    .bind(label)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StoreLayoutBody {
        layout_id: row.get("layout_id"),
        label: row.get("label"),
        sort_order: row.get("sort_order"),
    }))
}

pub async fn update_store_layout(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(layout_id): Path<i32>,
    Json(body): Json<UpdateLayoutRequest>,
) -> Result<StatusCode, StatusCode> {
    let label = body.label.trim();
    if label.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let affected =
        sqlx::query("UPDATE store_layouts SET label = $1, sort_order = $2 WHERE layout_id = $3")
            .bind(label)
            .bind(body.sort_order)
            .bind(layout_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .rows_affected();

    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_store_layout(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(layout_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let affected = sqlx::query("DELETE FROM store_layouts WHERE layout_id = $1")
        .bind(layout_id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .rows_affected();

    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn planner_products(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<Json<Vec<PlannerProductBody>>, StatusCode> {
    let rows = sqlx::query(
        "SELECT product_id, name, store_id, aisle_id FROM products WHERE store_id = $1 AND is_active = TRUE ORDER BY name ASC",
    )
    .bind(store_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let products = rows
        .into_iter()
        .map(|row| PlannerProductBody {
            product_id: row.get("product_id"),
            name: row.get("name"),
            store_id: row.get("store_id"),
            aisle_id: row.get("aisle_id"),
        })
        .collect();

    Ok(Json(products))
}

pub async fn create_store_product(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<CreateProductRequest>,
) -> Result<Json<PlannerProductBody>, StatusCode> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = sqlx::query(
        "INSERT INTO products (name, store_id) VALUES ($1, $2) RETURNING product_id, name, store_id, aisle_id",
    )
    .bind(name)
    .bind(store_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PlannerProductBody {
        product_id: row.get("product_id"),
        name: row.get("name"),
        store_id: row.get("store_id"),
        aisle_id: row.get("aisle_id"),
    }))
}

pub async fn assign_product_layout(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<AssignProductRequest>,
) -> Result<StatusCode, StatusCode> {
    if let Some(layout_id) = body.layout_id {
        let exists =
            sqlx::query("SELECT 1 FROM store_layouts WHERE layout_id = $1 AND store_id = $2")
                .bind(layout_id)
                .bind(store_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if exists.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let affected =
        sqlx::query("UPDATE products SET aisle_id = $1 WHERE product_id = $2 AND store_id = $3")
            .bind(body.layout_id)
            .bind(body.product_id)
            .bind(store_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .rows_affected();

    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
