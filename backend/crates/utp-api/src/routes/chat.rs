use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use utp_llm::{ChatRequest, Message};

#[derive(Deserialize)]
pub struct ChatRequestBody {
    message: String,
    model: Option<String>,
}

#[derive(Serialize)]
pub struct ChatResponseBody {
    response: String,
    model: String,
}

pub async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequestBody>,
) -> Json<ChatResponseBody> {
    let request = ChatRequest {
        model: payload.model.unwrap_or_else(|| "gpt-oss-20b".to_string()),
        messages: vec![Message {
            role: "user".to_string(),
            content: payload.message,
        }],
        temperature: None,
        max_tokens: None,
    };

    match state.llama_client.chat(request).await {
        Ok(response) => {
            let content = response.choices
                .first()
                .map(|choice| choice.message.content.clone())
                .unwrap_or_else(|| "No response".to_string());
            
            Json(ChatResponseBody {
                response: content,
                model: "gpt-oss-20b".to_string(),
            })
        },
        Err(_) => Json(ChatResponseBody {
            response: "Error processing request".to_string(),
            model: "error".to_string(),
        }),
    }
}