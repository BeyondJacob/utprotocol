use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct BenchmarkRequest {
    test_type: String,
    iterations: Option<u32>,
}

#[derive(Serialize)]
pub struct BenchmarkResponse {
    test_id: String,
    status: String,
}

#[derive(Serialize)]
pub struct BenchmarkResults {
    test_id: String,
    results: Vec<BenchmarkResult>,
}

#[derive(Serialize)]
pub struct BenchmarkResult {
    iteration: u32,
    latency_ms: f64,
    tokens_per_second: f64,
}

pub async fn run_benchmark(
    State(_state): State<AppState>,
    Json(payload): Json<BenchmarkRequest>,
) -> Json<BenchmarkResponse> {
    let test_id = format!("test_{}_{}", payload.test_type, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());
    
    // Use iterations in the response status
    let status = match payload.iterations {
        Some(iterations) => format!("started with {} iterations", iterations),
        None => "started with default iterations".to_string(),
    };
    
    Json(BenchmarkResponse {
        test_id,
        status,
    })
}

pub async fn get_results(State(state): State<AppState>) -> Json<BenchmarkResults> {
    // Use the message bus to get benchmark results (placeholder implementation)
    let _message_bus = &state.message_bus;
    
    Json(BenchmarkResults {
        test_id: "test_placeholder".to_string(),
        results: vec![],
    })
}