use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
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
    store_product_id: i32,
    standalone_product_id: i32,
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
pub struct UpdateStoreRequest {
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
    store_product_id: i32,
    layout_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateProductRequest {
    name: String,
}

#[derive(Deserialize)]
pub struct ProductSearchQuery {
    q: Option<String>,
}

#[derive(Serialize)]
pub struct ShoppingListItemBody {
    item_id: i32,
    store_product_id: i32,
    product_name: String,
    quantity: i32,
    aisle_id: Option<i32>,
    aisle_label: Option<String>,
    aisle_sort_order: Option<i32>,
}

#[derive(Serialize)]
pub struct ShoppingListSummaryBody {
    list_id: i32,
    status: String,
    created_at: String,
    closed_at: Option<String>,
    items: Vec<ShoppingListItemBody>,
}

#[derive(Serialize)]
pub struct StoreShoppingListBody {
    active_list: ShoppingListSummaryBody,
    past_lists: Vec<ShoppingListSummaryBody>,
}

#[derive(Deserialize)]
pub struct AddShoppingListItemRequest {
    store_product_id: i32,
    quantity: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateShoppingListItemRequest {
    quantity: i32,
}

#[derive(Deserialize)]
pub struct CreateStandaloneProductRequest {
    name: String,
}

#[derive(Serialize)]
pub struct StandaloneProductBody {
    standalone_product_id: i32,
    name: String,
}

#[derive(Deserialize)]
pub struct AddStoreProductFromStandaloneRequest {
    standalone_product_id: i32,
    aisle_id: Option<i32>,
}

async fn ensure_active_list_for_store(
    pool: &PgPool,
    store_id: i32,
    user_id: i32,
) -> Result<i32, StatusCode> {
    let maybe_active = sqlx::query(
        "SELECT list_id FROM store_shopping_lists WHERE store_id = $1 AND status = 'active' ORDER BY list_id DESC LIMIT 1",
    )
    .bind(store_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(row) = maybe_active {
        return Ok(row.get("list_id"));
    }

    let created = sqlx::query(
        "INSERT INTO store_shopping_lists (store_id, created_by, status) VALUES ($1, $2, 'active') RETURNING list_id",
    )
    .bind(store_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(created.get("list_id"))
}

fn group_list_rows(rows: Vec<sqlx::postgres::PgRow>) -> Vec<ShoppingListSummaryBody> {
    let mut map: HashMap<i32, ShoppingListSummaryBody> = HashMap::new();
    for row in rows {
        let list_id: i32 = row.get("list_id");
        let entry = map
            .entry(list_id)
            .or_insert_with(|| ShoppingListSummaryBody {
                list_id,
                status: row.get("status"),
                created_at: row.get::<time::OffsetDateTime, _>("created_at").to_string(),
                closed_at: row
                    .get::<Option<time::OffsetDateTime>, _>("closed_at")
                    .map(|value| value.to_string()),
                items: Vec::new(),
            });

        let maybe_item_id: Option<i32> = row.get("item_id");
        if let Some(item_id) = maybe_item_id {
            entry.items.push(ShoppingListItemBody {
                item_id,
                store_product_id: row.get("store_product_id"),
                product_name: row.get("product_name"),
                quantity: row.get("quantity"),
                aisle_id: row.get("aisle_id"),
                aisle_label: row.get("aisle_label"),
                aisle_sort_order: row.get("aisle_sort_order"),
            });
        }
    }

    let mut lists: Vec<ShoppingListSummaryBody> = map.into_values().collect();
    lists.sort_by(|a, b| b.list_id.cmp(&a.list_id));
    lists
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

pub async fn update_planner_store(
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<UpdateStoreRequest>,
) -> Result<StatusCode, StatusCode> {
    let name = body.name.trim();
    let address = body.address.trim();
    if name.is_empty() || address.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let affected = sqlx::query("UPDATE stores SET name = $1, address = $2 WHERE store_id = $3")
        .bind(name)
        .bind(address)
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

pub async fn delete_planner_store(
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let affected = sqlx::query("DELETE FROM stores WHERE store_id = $1")
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
        r#"
        SELECT
            sp.store_product_id,
            sp.store_id,
            sp.standalone_product_id,
            sp.aisle_id,
            p.name
        FROM store_products sp
        JOIN standalone_products p ON p.standalone_product_id = sp.standalone_product_id
        WHERE sp.store_id = $1 AND sp.is_active = TRUE
        ORDER BY p.name ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let store_products = rows
        .into_iter()
        .map(|row| PlannerProductBody {
            store_product_id: row.get("store_product_id"),
            standalone_product_id: row.get("standalone_product_id"),
            name: row.get("name"),
            store_id: row.get("store_id"),
            aisle_id: row.get("aisle_id"),
        })
        .collect();

    Ok(Json(store_products))
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

    let standalone = sqlx::query(
        "INSERT INTO standalone_products (name) VALUES ($1)
         ON CONFLICT (name) DO UPDATE SET is_active = TRUE
         RETURNING standalone_product_id, name",
    )
    .bind(name)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let standalone_product_id: i32 = standalone.get("standalone_product_id");
    let row = sqlx::query(
        "INSERT INTO store_products (store_id, standalone_product_id) VALUES ($1, $2)
         ON CONFLICT (store_id, standalone_product_id) DO UPDATE SET is_active = TRUE
         RETURNING store_product_id, store_id, standalone_product_id, aisle_id",
    )
    .bind(store_id)
    .bind(standalone_product_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PlannerProductBody {
        store_product_id: row.get("store_product_id"),
        standalone_product_id: row.get("standalone_product_id"),
        name: standalone.get("name"),
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

    let affected = sqlx::query(
        "UPDATE store_products SET aisle_id = $1 WHERE store_product_id = $2 AND store_id = $3",
    )
    .bind(body.layout_id)
    .bind(body.store_product_id)
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

pub async fn standalone_products(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Query(query): Query<ProductSearchQuery>,
) -> Result<Json<Vec<StandaloneProductBody>>, StatusCode> {
    let pattern = format!("%{}%", query.q.unwrap_or_default().trim());
    let rows = sqlx::query(
        "SELECT standalone_product_id, name FROM standalone_products WHERE is_active = TRUE AND name ILIKE $1 ORDER BY name ASC LIMIT 100",
    )
    .bind(pattern)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|row| StandaloneProductBody {
                standalone_product_id: row.get("standalone_product_id"),
                name: row.get("name"),
            })
            .collect(),
    ))
}

pub async fn create_standalone_product(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Json(body): Json<CreateStandaloneProductRequest>,
) -> Result<Json<StandaloneProductBody>, StatusCode> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = sqlx::query(
        "INSERT INTO standalone_products (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET is_active = TRUE RETURNING standalone_product_id, name",
    )
    .bind(name)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StandaloneProductBody {
        standalone_product_id: row.get("standalone_product_id"),
        name: row.get("name"),
    }))
}

pub async fn create_store_product_from_standalone(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<AddStoreProductFromStandaloneRequest>,
) -> Result<Json<PlannerProductBody>, StatusCode> {
    let standalone =
        sqlx::query("SELECT name FROM standalone_products WHERE standalone_product_id = $1")
            .bind(body.standalone_product_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(standalone_row) = standalone else {
        return Err(StatusCode::NOT_FOUND);
    };

    let name: String = standalone_row.get("name");
    let row = sqlx::query(
        "INSERT INTO store_products (store_id, standalone_product_id, aisle_id) VALUES ($1, $2, $3)
         ON CONFLICT (store_id, standalone_product_id) DO UPDATE
         SET aisle_id = COALESCE(EXCLUDED.aisle_id, store_products.aisle_id), is_active = TRUE
         RETURNING store_product_id, store_id, standalone_product_id, aisle_id",
    )
    .bind(store_id)
    .bind(body.standalone_product_id)
    .bind(body.aisle_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PlannerProductBody {
        store_product_id: row.get("store_product_id"),
        standalone_product_id: row.get("standalone_product_id"),
        name,
        store_id: row.get("store_id"),
        aisle_id: row.get("aisle_id"),
    }))
}

pub async fn store_shopping_list(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<Json<StoreShoppingListBody>, StatusCode> {
    let active_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            l.list_id,
            l.status,
            l.created_at,
            l.closed_at,
            i.item_id,
            i.store_product_id,
            i.quantity,
            p.name AS product_name,
            sp.aisle_id,
            a.label AS aisle_label,
            a.sort_order AS aisle_sort_order
        FROM store_shopping_lists l
        LEFT JOIN store_shopping_list_items i ON i.list_id = l.list_id
        LEFT JOIN store_products sp ON sp.store_product_id = i.store_product_id
        LEFT JOIN standalone_products p ON p.standalone_product_id = sp.standalone_product_id
        LEFT JOIN store_layouts a ON a.layout_id = sp.aisle_id
        WHERE l.store_id = $1
        ORDER BY
            l.created_at DESC,
            COALESCE(a.sort_order, 2147483647) ASC,
            p.name ASC,
            i.created_at ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut lists = group_list_rows(rows);
    let active_index = lists.iter().position(|l| l.list_id == active_id);
    let active_list = active_index
        .map(|idx| lists.remove(idx))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StoreShoppingListBody {
        active_list,
        past_lists: lists.into_iter().filter(|l| l.status == "closed").collect(),
    }))
}

pub async fn add_store_shopping_list_item(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
    Json(body): Json<AddShoppingListItemRequest>,
) -> Result<StatusCode, StatusCode> {
    let list_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;
    let quantity = body.quantity.unwrap_or(1).max(1);

    sqlx::query(
        r#"
        INSERT INTO store_shopping_list_items (list_id, store_product_id, quantity)
        VALUES ($1, $2, $3)
        ON CONFLICT (list_id, store_product_id)
        DO UPDATE SET quantity = store_shopping_list_items.quantity + EXCLUDED.quantity
        "#,
    )
    .bind(list_id)
    .bind(body.store_product_id)
    .bind(quantity)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn close_store_shopping_list(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let list_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;

    sqlx::query(
        "UPDATE store_shopping_lists SET status = 'closed', closed_at = NOW() WHERE list_id = $1",
    )
    .bind(list_id)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_store_shopping_list_item(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path((store_id, item_id)): Path<(i32, i32)>,
    Json(body): Json<UpdateShoppingListItemRequest>,
) -> Result<StatusCode, StatusCode> {
    if body.quantity < 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let active_list_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;
    let affected = sqlx::query(
        r#"
        UPDATE store_shopping_list_items i
        SET quantity = $1
        FROM store_shopping_lists l
        WHERE i.item_id = $2
          AND l.list_id = i.list_id
          AND l.store_id = $3
          AND l.list_id = $4
          AND l.status = 'active'
        "#,
    )
    .bind(body.quantity)
    .bind(item_id)
    .bind(store_id)
    .bind(active_list_id)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .rows_affected();

    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_store_shopping_list_item(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path((store_id, item_id)): Path<(i32, i32)>,
) -> Result<StatusCode, StatusCode> {
    let active_list_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;
    let affected = sqlx::query(
        r#"
        DELETE FROM store_shopping_list_items i
        USING store_shopping_lists l
        WHERE i.item_id = $1
          AND l.list_id = i.list_id
          AND l.store_id = $2
          AND l.list_id = $3
          AND l.status = 'active'
        "#,
    )
    .bind(item_id)
    .bind(store_id)
    .bind(active_list_id)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .rows_affected();

    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
