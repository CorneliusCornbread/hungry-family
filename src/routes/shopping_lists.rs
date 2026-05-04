use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;

use crate::auth::CurrentAccount;

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

async fn ensure_active_list_for_store(
    pool: &PgPool,
    store_id: i32,
    user_id: i32,
) -> Result<i32, StatusCode> {
    let maybe_active = sqlx::query("SELECT list_id FROM store_shopping_lists WHERE store_id = $1 AND status = 'active' ORDER BY list_id DESC LIMIT 1").bind(store_id).fetch_optional(pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(row) = maybe_active {
        return Ok(row.get("list_id"));
    }
    let created = sqlx::query("INSERT INTO store_shopping_lists (store_id, created_by, status) VALUES ($1, $2, 'active') RETURNING list_id").bind(store_id).bind(user_id).fetch_one(pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
                    .map(|v| v.to_string()),
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

pub async fn store_shopping_list(
    CurrentAccount(account): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<Json<StoreShoppingListBody>, StatusCode> {
    let active_id = ensure_active_list_for_store(&pool, store_id, account.user_id).await?;
    let rows = sqlx::query(r#"SELECT l.list_id,l.status,l.created_at,l.closed_at,i.item_id,i.store_product_id,i.quantity,p.name AS product_name,sp.aisle_id,a.label AS aisle_label,a.sort_order AS aisle_sort_order FROM store_shopping_lists l LEFT JOIN store_shopping_list_items i ON i.list_id = l.list_id LEFT JOIN store_products sp ON sp.store_product_id = i.store_product_id LEFT JOIN standalone_products p ON p.standalone_product_id = sp.standalone_product_id LEFT JOIN store_layouts a ON a.layout_id = sp.aisle_id WHERE l.store_id = $1 ORDER BY l.created_at DESC, COALESCE(a.sort_order, 2147483647) ASC, p.name ASC, i.created_at ASC"#).bind(store_id).fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    sqlx::query(r#"INSERT INTO store_shopping_list_items (list_id, store_product_id, quantity) VALUES ($1, $2, $3) ON CONFLICT (list_id, store_product_id) DO UPDATE SET quantity = store_shopping_list_items.quantity + EXCLUDED.quantity"#).bind(list_id).bind(body.store_product_id).bind(quantity).execute(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let affected = sqlx::query(r#"UPDATE store_shopping_list_items i SET quantity = $1 FROM store_shopping_lists l WHERE i.item_id = $2 AND l.list_id = i.list_id AND l.store_id = $3 AND l.list_id = $4 AND l.status = 'active'"#).bind(body.quantity).bind(item_id).bind(store_id).bind(active_list_id).execute(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.rows_affected();
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
    let affected = sqlx::query(r#"DELETE FROM store_shopping_list_items i USING store_shopping_lists l WHERE i.item_id = $1 AND l.list_id = i.list_id AND l.store_id = $2 AND l.list_id = $3 AND l.status = 'active'"#).bind(item_id).bind(store_id).bind(active_list_id).execute(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.rows_affected();
    if affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
