// crates/utp-api/src/main.rs
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber;

mod routes;
mod state;

use routes::{health, chat, benchmark, agents};
use state::AppState;

#[tokio::main]
async fn main() {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Create shared state
    let state = AppState::new();

    // Build router with all endpoints
    let app = Router::new()
        .route("/api/health", get(health::health_check))
        .route("/api/chat", post(chat::chat_handler))
        .route("/api/benchmark/run", post(benchmark::run_benchmark))
        .route("/api/benchmark/results", get(benchmark::get_results))
        .route("/api/agents", get(agents::list_agents))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}