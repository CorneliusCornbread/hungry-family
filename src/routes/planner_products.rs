use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::CurrentAccount;

#[derive(Serialize)]
pub struct PlannerProductBody {
    store_product_id: i32,
    standalone_product_id: i32,
    name: String,
    store_id: i32,
    aisle_id: Option<i32>,
}
#[derive(Deserialize)]
pub struct CreateProductRequest {
    name: String,
}
#[derive(Deserialize)]
pub struct AssignProductRequest {
    store_product_id: i32,
    layout_id: Option<i32>,
}
#[derive(Deserialize)]
pub struct ProductSearchQuery {
    q: Option<String>,
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

pub async fn planner_products(
    CurrentAccount(_): CurrentAccount,
    State(pool): State<PgPool>,
    Path(store_id): Path<i32>,
) -> Result<Json<Vec<PlannerProductBody>>, StatusCode> {
    let rows = sqlx::query(r#"SELECT sp.store_product_id,sp.store_id,sp.standalone_product_id,sp.aisle_id,p.name FROM store_products sp JOIN standalone_products p ON p.standalone_product_id = sp.standalone_product_id WHERE sp.store_id = $1 AND sp.is_active = TRUE ORDER BY p.name ASC"#).bind(store_id).fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| PlannerProductBody {
                store_product_id: row.get("store_product_id"),
                standalone_product_id: row.get("standalone_product_id"),
                name: row.get("name"),
                store_id: row.get("store_id"),
                aisle_id: row.get("aisle_id"),
            })
            .collect(),
    ))
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
    let standalone = sqlx::query("INSERT INTO standalone_products (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET is_active = TRUE RETURNING standalone_product_id, name").bind(name).fetch_one(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let standalone_product_id: i32 = standalone.get("standalone_product_id");
    let row = sqlx::query("INSERT INTO store_products (store_id, standalone_product_id) VALUES ($1, $2) ON CONFLICT (store_id, standalone_product_id) DO UPDATE SET is_active = TRUE RETURNING store_product_id, store_id, standalone_product_id, aisle_id").bind(store_id).bind(standalone_product_id).fetch_one(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let rows = sqlx::query("SELECT standalone_product_id, name FROM standalone_products WHERE is_active = TRUE AND name ILIKE $1 ORDER BY name ASC LIMIT 100").bind(pattern).fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let row = sqlx::query("INSERT INTO standalone_products (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET is_active = TRUE RETURNING standalone_product_id, name").bind(name).fetch_one(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let row = sqlx::query("INSERT INTO store_products (store_id, standalone_product_id, aisle_id) VALUES ($1, $2, $3) ON CONFLICT (store_id, standalone_product_id) DO UPDATE SET aisle_id = COALESCE(EXCLUDED.aisle_id, store_products.aisle_id), is_active = TRUE RETURNING store_product_id, store_id, standalone_product_id, aisle_id").bind(store_id).bind(body.standalone_product_id).bind(body.aisle_id).fetch_one(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(PlannerProductBody {
        store_product_id: row.get("store_product_id"),
        standalone_product_id: row.get("standalone_product_id"),
        name,
        store_id: row.get("store_id"),
        aisle_id: row.get("aisle_id"),
    }))
}
