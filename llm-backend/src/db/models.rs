use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: i32,
    pub title: String,
    pub model: String,
    pub utp_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: i32,
    pub conversation_id: i32,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub latency_ms: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub tokens_per_second: Option<f64>,
    pub total_tokens: Option<i32>,
    pub utp_metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: String,
    pub model: String,
    #[serde(default)]
    pub utp_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ConversationWithMessages {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}
