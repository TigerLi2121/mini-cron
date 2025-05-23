mod common;
mod mid;
mod model;
mod route;

use std::time::Duration;

use axum::{Router, extract::State, middleware, routing::get};
use common::log::init_log;
use reqwest::Client;
use tracing::info;

#[derive(Clone)]
struct AppState {
    http: Client,
    task: common::task::CronManager,
}

#[tokio::main]
async fn main() {
    init_log().await;

    let state = AppState {
        http: Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client"),
        task: common::task::CronManager::new(),
    };

    let app = Router::new()
        .route("/", get(|_: State<AppState>| async { "Hello, World!" }))
        .nest("/task", route::task::router())
        .layer(middleware::from_fn(mid::api_log::log))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
