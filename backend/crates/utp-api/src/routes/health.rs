use axum::{Json, extract::State};
use serde::Serialize;
use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    models_loaded: Vec<String>,
}

pub async fn health_check(State(_state): State<AppState>) -> Json<HealthResponse> {
    // TODO: Actually check if llama.cpp is responding
    Json(HealthResponse {
        status: "ok".to_string(),
        models_loaded: vec!["gpt-oss-20b".to_string()],
    })
}