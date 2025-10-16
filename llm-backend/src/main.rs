use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

mod db;
use db::{
    models::{Conversation, ConversationWithMessages, CreateConversationRequest, Message},
    repository::{ConversationRepository, MessageRepository},
};

// Shared application state
#[derive(Clone)]
struct AppState {
    active_model: Arc<RwLock<String>>,
    ollama_client: reqwest::Client,
    db_pool: PgPool,
}

// Request/Response types
#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    message: String,
    conversation_id: Option<i32>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    model: String,
    latency_ms: u128,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    name: String,
    downloaded: bool,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
    active_model: String,
}

#[derive(Debug, Deserialize)]
struct SwitchModelRequest {
    model: String,
}

#[derive(Debug, Deserialize)]
struct ModelActionRequest {
    model: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    ollama_connected: bool,
}

// Ollama API types
#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaListResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize)]
struct OllamaPullRequest {
    name: String,
    stream: bool,
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::from_filename("../.env").ok();

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://jacobowens@localhost/utprotocol".to_string());

    let db_pool = db::init_db(&database_url)
        .await
        .expect("Failed to initialize database");

    println!("✅ Database connected and initialized");

    // Initialize state
    let state = AppState {
        active_model: Arc::new(RwLock::new("gpt-oss:20b".to_string())),
        ollama_client: reqwest::Client::new(),
        db_pool,
    };

    // Configure CORS for Next.js frontend
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/models", get(models_handler))
        .route("/chat", post(chat_handler))
        .route("/switch-model", post(switch_model_handler))
        .route("/download-model", post(download_model_handler))
        .route("/delete-model", post(delete_model_handler))
        // Conversation routes
        .route("/conversations", get(get_conversations_handler))
        .route("/conversations", post(create_conversation_handler))
        .route("/conversations/:id", get(get_conversation_handler))
        .route("/conversations/:id", delete(delete_conversation_handler))
        .with_state(state)
        .layer(cors);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 Server running on http://127.0.0.1:3001");

    axum::serve(listener, app).await.unwrap();
}

// Health check handler
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Check if Ollama is accessible
    let ollama_connected = state
        .ollama_client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .is_ok();

    Json(HealthResponse {
        status: "ok".to_string(),
        ollama_connected,
    })
}

// List available models handler
async fn models_handler(State(state): State<AppState>) -> impl IntoResponse {
    let active_model = state.active_model.read().await.clone();

    // Get list of downloaded models from Ollama
    let downloaded_models = match state
        .ollama_client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                response
                    .json::<OllamaListResponse>()
                    .await
                    .map(|list| {
                        list.models
                            .into_iter()
                            .map(|m| m.name)
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_else(|_| vec![])
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    };

    // All available models for UTP
    let all_model_names = vec![
        "gpt-oss:20b",
        "gpt-oss:120b",
        "llama3.2:1b",
        "llama3.2:3b",
    ];

    let models: Vec<ModelInfo> = all_model_names
        .into_iter()
        .map(|name| ModelInfo {
            name: name.to_string(),
            downloaded: downloaded_models.contains(&name.to_string()),
        })
        .collect();

    Json(ModelsResponse {
        models,
        active_model,
    })
}

// Chat handler
async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let start = std::time::Instant::now();

    // Save user message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());
        message_repo
            .create_message(conversation_id, "user", &payload.message, None)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save user message: {}", e),
                )
            })?;

        // Update conversation timestamp
        let conv_repo = ConversationRepository::new(state.db_pool.clone());
        conv_repo
            .update_conversation_timestamp(conversation_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update conversation timestamp: {}", e),
                )
            })?;
    }

    // Call Ollama API
    let ollama_request = OllamaGenerateRequest {
        model: payload.model.clone(),
        prompt: payload.message,
        stream: false,
    };

    let response = state
        .ollama_client
        .post("http://localhost:11434/api/generate")
        .json(&ollama_request)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to call Ollama API: {}", e),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Ollama API returned error: {}", response.status()),
        ));
    }

    let ollama_response: OllamaGenerateResponse = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse Ollama response: {}", e),
        )
    })?;

    let latency_ms = start.elapsed().as_millis();

    // Save assistant message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());
        message_repo
            .create_message(
                conversation_id,
                "assistant",
                &ollama_response.response,
                Some(latency_ms as i32),
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save assistant message: {}", e),
                )
            })?;
    }

    Ok(Json(ChatResponse {
        response: ollama_response.response,
        model: payload.model,
        latency_ms,
    }))
}

// Switch model handler
async fn switch_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<SwitchModelRequest>,
) -> impl IntoResponse {
    let mut active_model = state.active_model.write().await;
    *active_model = payload.model.clone();

    Json(serde_json::json!({
        "success": true,
        "model": payload.model
    }))
}

// Download model handler
async fn download_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<ModelActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    println!("📥 Downloading model: {}", payload.model);

    let pull_request = OllamaPullRequest {
        name: payload.model.clone(),
        stream: false,
    };

    // Call Ollama pull API
    let response = state
        .ollama_client
        .post("http://localhost:11434/api/pull")
        .json(&pull_request)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to call Ollama pull API: {}", e),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Ollama pull API returned error: {}", response.status()),
        ));
    }

    println!("✅ Model download initiated: {}", payload.model);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Model {} download started", payload.model)
    })))
}

// Delete model handler
async fn delete_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<ModelActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    println!("🗑️ Deleting model: {}", payload.model);

    // Call Ollama delete API
    let response = state
        .ollama_client
        .delete(format!("http://localhost:11434/api/delete"))
        .json(&serde_json::json!({
            "name": payload.model
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to call Ollama delete API: {}", e),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Ollama delete API returned error: {}", response.status()),
        ));
    }

    println!("✅ Model deleted: {}", payload.model);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Model {} deleted", payload.model)
    })))
}

// Conversation handlers
async fn get_conversations_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo = ConversationRepository::new(state.db_pool.clone());

    let conversations = repo.get_all_conversations().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch conversations: {}", e),
        )
    })?;

    Ok(Json(conversations))
}

async fn create_conversation_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateConversationRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo = ConversationRepository::new(state.db_pool.clone());

    let conversation = repo
        .create_conversation(&payload.title, &payload.model)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create conversation: {}", e),
            )
        })?;

    Ok(Json(conversation))
}

async fn get_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conv_repo = ConversationRepository::new(state.db_pool.clone());
    let msg_repo = MessageRepository::new(state.db_pool.clone());

    let conversation = conv_repo
        .get_conversation(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch conversation: {}", e),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Conversation not found".to_string()))?;

    let messages = msg_repo
        .get_messages_by_conversation(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch messages: {}", e),
            )
        })?;

    Ok(Json(ConversationWithMessages {
        conversation,
        messages,
    }))
}

async fn delete_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo = ConversationRepository::new(state.db_pool.clone());

    repo.delete_conversation(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete conversation: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Conversation deleted"
    })))
}
