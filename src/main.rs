use axum::{Router, routing::get};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new().fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:800")
        .await
        .expect("Unable to bind to port 800");
    axum::serve(listener, app).await.unwrap();
}
