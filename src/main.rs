#[allow(dead_code)]
mod auth;

use askama::Template;
use axum::{Router, response::Html, routing::get};
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> Html<String> {
    Html(IndexTemplate.render().unwrap())
}

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

    let app = Router::new()
        .route("/", get(index))
        .fallback_service(ServeDir::new("static"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:800")
        .await
        .expect("Unable to bind to port 800");

    tracing::info!("Serving on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
