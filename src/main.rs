use askama::Template;
use axum::extract::{FromRef, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Router, response::Html, routing::get};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

mod auth;
mod db;
mod setup;

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pool: PgPool,
    setup_required: Arc<AtomicBool>,
}

impl AppState {
    async fn new(pg_pool: PgPool) -> Self {
        let setup_required = setup::setup_required(&pg_pool)
            .await
            .expect("Could not query database for app state setup");

        Self {
            pool: pg_pool,
            setup_required: Arc::new(AtomicBool::new(setup_required)),
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index(State(app_state): State<AppState>) -> Response {
    if app_state.setup_required.load(Ordering::Acquire) {
        return Redirect::to("/setup").into_response();
    }

    Html(IndexTemplate.render().unwrap()).into_response()
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

    db::ensure_database(&database_url)
        .await
        .expect("Failed to create database");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .idle_timeout(IDLE_TIMEOUT)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database ready");

    let app_state = AppState::new(pool).await;

    let app = Router::new()
        .route("/", get(index))
        .route("/setup", get(setup::get_setup).post(setup::post_setup))
        .fallback_service(ServeDir::new("static"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:800")
        .await
        .expect("Unable to bind to port 800");

    tracing::info!("Serving on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
