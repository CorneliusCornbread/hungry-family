mod auth;
mod routes;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
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
        .route("/api/auth/logout", post(routes::logout));

    let app = api
        // In production the Vite build outputs to ./static — serve it here.
        // All unmatched routes fall back to index.html for client-side routing.
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:800")
        .await
        .expect("Unable to bind to port 800");

    tracing::info!("Serving on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
