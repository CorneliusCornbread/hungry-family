mod auth;
mod routes;

use axum::{
    Router,
    routing::{get, patch, post},
};
use sqlx::postgres::PgPoolOptions;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    tracing::info!("Connected to database");

    let api = Router::new()
        .route("/api/auth/me", get(routes::me))
        .route("/api/auth/login", post(routes::login))
        .route("/api/auth/logout", post(routes::logout))
        .route("/api/planner/stores", get(routes::planner_stores))
        .route("/api/planner/stores", post(routes::create_planner_store))
        .route(
            "/api/planner/stores/{store_id}/layouts",
            post(routes::create_store_layout),
        )
        .route(
            "/api/planner/layouts/{layout_id}",
            patch(routes::update_store_layout).delete(routes::delete_store_layout),
        )
        .route(
            "/api/planner/stores/{store_id}/products",
            get(routes::planner_products),
        )
        .route(
            "/api/planner/stores/{store_id}/product-layout",
            patch(routes::assign_product_layout),
        );

    let app = api
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:800")
        .await
        .expect("Unable to bind to port 800");

    tracing::info!("Serving on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
