use axum::{routing::get, Router};
use std::net::SocketAddr;

async fn dashboard() -> &'static str {
    include_str!("../static/dashboard.html")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(dashboard));
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Listening on http://127.0.0.1:8080");
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app.into_make_service()).await.unwrap();
}
