use axum::{Json, extract::State};
use serde::Serialize;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AgentInfo {
    id: String,
    name: String,
    status: String,
}

#[derive(Serialize)]
pub struct AgentsResponse {
    agents: Vec<AgentInfo>,
}

pub async fn list_agents(State(_state): State<AppState>) -> Json<AgentsResponse> {
    // TODO: Get actual agents from the message bus
    let agents = vec![
        AgentInfo {
            id: "agent_1".to_string(),
            name: "Default Agent".to_string(),
            status: "active".to_string(),
        },
    ];

    Json(AgentsResponse { agents })
}