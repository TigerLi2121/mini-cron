mod common;
mod mid;

use axum::{Router, middleware, routing::get};
use common::log::init_log;
use tracing::info;

#[tokio::main]
async fn main() {
    init_log().await;
    common::task::CronManager::new();

    let router = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(middleware::from_fn(mid::api_log::log));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}
