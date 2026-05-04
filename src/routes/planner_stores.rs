use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::CurrentAccount;

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

pub async fn planner_stores(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<PlannerStoreBody>>, StatusCode> {
    let store_rows = sqlx::query("SELECT store_id, name, address FROM stores ORDER BY name ASC")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let layout_rows = sqlx::query("SELECT layout_id, store_id, label, sort_order FROM store_layouts ORDER BY store_id ASC, sort_order ASC, layout_id ASC").fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    let row = sqlx::query(r#"INSERT INTO store_layouts (store_id, label, sort_order) VALUES ($1,$2,COALESCE((SELECT MAX(sort_order) + 1 FROM store_layouts WHERE store_id = $1), 1)) RETURNING layout_id, label, sort_order"#).bind(store_id).bind(label).fetch_one(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
