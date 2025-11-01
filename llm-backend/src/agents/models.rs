use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    Input,
    Responder,
    Analyzer,
    Router,
    Aggregator,
}

impl AgentType {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            AgentType::Input => "input",
            AgentType::Responder => "responder",
            AgentType::Analyzer => "analyzer",
            AgentType::Router => "router",
            AgentType::Aggregator => "aggregator",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "input" => Ok(AgentType::Input),
            "responder" => Ok(AgentType::Responder),
            "analyzer" => Ok(AgentType::Analyzer),
            "router" => Ok(AgentType::Router),
            "aggregator" => Ok(AgentType::Aggregator),
            _ => Err(format!("Unknown agent type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Running,
    Completed,
    Error,
}

impl AgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            AgentStatus::Idle => "idle",
            AgentStatus::Running => "running",
            AgentStatus::Completed => "completed",
            AgentStatus::Error => "error",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "idle" => Ok(AgentStatus::Idle),
            "running" => Ok(AgentStatus::Running),
            "completed" => Ok(AgentStatus::Completed),
            "error" => Ok(AgentStatus::Error),
            _ => Err(format!("Unknown agent status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: i32,
    pub label: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub status: String,
    pub model: Option<String>,
    pub utp_enabled: bool,
    pub config: serde_json::Value,
    pub position_x: f64,
    pub position_y: f64,
    pub last_output: Option<String>,
    pub last_error: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFlow {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEdge {
    pub id: i32,
    pub flow_id: i32,
    pub source_agent_id: i32,
    pub target_agent_id: i32,
    pub edge_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: i32,
    pub source_agent_id: i32,
    pub target_agent_id: i32,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub label: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub model: Option<String>,
    #[serde(default)]
    pub utp_enabled: bool,
    pub config: Option<serde_json::Value>,
    pub position: Position,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub label: Option<String>,
    pub status: Option<String>,
    pub model: Option<String>,
    pub utp_enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
    pub position: Option<Position>,
    pub last_output: Option<String>,
    pub last_error: Option<String>,
    pub execution_time_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFlowRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub flow_id: i32,
    pub source_agent_id: i32,
    pub target_agent_id: i32,
    pub edge_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteAgentRequest {
    pub input: String,
    #[allow(dead_code)]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteAgentResponse {
    pub agent_id: i32,
    pub output: String,
    pub status: String,
    pub execution_time_ms: i32,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct FlowWithAgents {
    pub flow: AgentFlow,
    pub agents: Vec<Agent>,
    pub edges: Vec<AgentEdge>,
}
