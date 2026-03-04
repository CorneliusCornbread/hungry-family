use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello World! " }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80")
        .await
        .expect("Unable to bind to port 80");
    axum::serve(listener, app).await.unwrap();
}
