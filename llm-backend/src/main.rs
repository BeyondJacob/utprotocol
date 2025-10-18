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
mod providers;

use db::{
    models::{ConversationWithMessages, CreateConversationRequest},
    repository::{ConversationRepository, MessageRepository},
};
use providers::{
    groq::GroqProvider,
    ollama::OllamaProvider,
    ModelProvider, ModelRegistry, ProviderType,
};

// Shared application state
#[derive(Clone)]
struct AppState {
    active_model: Arc<RwLock<String>>,
    ollama_provider: Arc<OllamaProvider>,
    groq_provider: Arc<GroqProvider>,
    model_registry: Arc<ModelRegistry>,
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
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    tokens_per_second: Option<f64>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ModelPricingInfo {
    input_per_million: f64,
    output_per_million: f64,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    name: String,
    display_name: String,
    provider: String,
    is_local: bool,
    downloaded: bool,
    pricing: Option<ModelPricingInfo>,
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

    // Initialize HTTP client
    let http_client = reqwest::Client::new();

    // Initialize providers
    let groq_api_key = std::env::var("GROQ_API_KEY").ok();
    let ollama_provider = Arc::new(OllamaProvider::new(http_client.clone()));
    let groq_provider = Arc::new(GroqProvider::new(http_client.clone(), groq_api_key.clone()));
    let model_registry = Arc::new(ModelRegistry::new());

    // Check provider status
    if ollama_provider.is_connected().await {
        println!("✅ Ollama provider connected");
    } else {
        println!("⚠️  Ollama provider not connected");
    }

    if groq_provider.is_connected().await {
        println!("✅ Groq provider configured");
    } else {
        println!("⚠️  Groq provider not configured (GROQ_API_KEY missing)");
    }

    // Initialize state
    let state = AppState {
        active_model: Arc::new(RwLock::new("gpt-oss:20b".to_string())),
        ollama_provider,
        groq_provider,
        model_registry,
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
    let ollama_connected = state.ollama_provider.is_connected().await;
    let groq_connected = state.groq_provider.is_connected().await;

    Json(serde_json::json!({
        "status": "ok",
        "providers": {
            "ollama": ollama_connected,
            "groq": groq_connected,
        }
    }))
}

// List available models handler
async fn models_handler(State(state): State<AppState>) -> impl IntoResponse {
    let active_model = state.active_model.read().await.clone();

    // Get list of downloaded Ollama models
    let downloaded_models = state
        .ollama_provider
        .list_models()
        .await
        .unwrap_or_else(|_| vec![]);

    // Build model list from registry
    let models: Vec<ModelInfo> = state
        .model_registry
        .all_models()
        .iter()
        .map(|metadata| {
            let downloaded = if metadata.is_local {
                downloaded_models.contains(&metadata.name)
            } else {
                // Cloud models are always "available" if provider is configured
                match metadata.provider {
                    ProviderType::Groq => state.groq_provider.is_configured(),
                    ProviderType::Ollama => true,
                }
            };

            ModelInfo {
                name: metadata.name.clone(),
                display_name: metadata.display_name.clone(),
                provider: metadata.provider.as_str().to_string(),
                is_local: metadata.is_local,
                downloaded,
                pricing: metadata.pricing.as_ref().map(|p| ModelPricingInfo {
                    input_per_million: p.input_per_million,
                    output_per_million: p.output_per_million,
                }),
            }
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
    // Save user message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());
        message_repo
            .create_message(conversation_id, "user", &payload.message, Some(&payload.model), None, None, None, None, None)
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

    // Determine which provider to use based on model
    let model_metadata = state
        .model_registry
        .get_model(&payload.model)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("Unknown model: {}", payload.model),
            )
        })?;

    // Route to appropriate provider
    let generate_response = match model_metadata.provider {
        ProviderType::Ollama => state
            .ollama_provider
            .generate(&payload.model, &payload.message)
            .await,
        ProviderType::Groq => state
            .groq_provider
            .generate(&payload.model, &payload.message)
            .await,
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate response: {}", e),
        )
    })?;

    let latency_ms = generate_response.latency_ms;

    // Save assistant message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());
        message_repo
            .create_message(
                conversation_id,
                "assistant",
                &generate_response.content,
                Some(&payload.model),
                Some(latency_ms as i32),
                generate_response.prompt_tokens.map(|t| t as i32),
                generate_response.completion_tokens.map(|t| t as i32),
                generate_response.tokens_per_second,
                generate_response.total_tokens.map(|t| t as i32),
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
        response: generate_response.content,
        model: payload.model,
        latency_ms,
        prompt_tokens: generate_response.prompt_tokens,
        completion_tokens: generate_response.completion_tokens,
        tokens_per_second: generate_response.tokens_per_second,
        total_tokens: generate_response.total_tokens,
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

// Download model handler (Ollama only)
async fn download_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<ModelActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if model is a local Ollama model
    let model_metadata = state
        .model_registry
        .get_model(&payload.model)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("Unknown model: {}", payload.model),
            )
        })?;

    if !model_metadata.is_local {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot download cloud models".to_string(),
        ));
    }

    println!("📥 Downloading model: {}", payload.model);

    state
        .ollama_provider
        .pull_model(&payload.model)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to download model: {}", e),
            )
        })?;

    println!("✅ Model download initiated: {}", payload.model);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Model {} download started", payload.model)
    })))
}

// Delete model handler (Ollama only)
async fn delete_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<ModelActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if model is a local Ollama model
    let model_metadata = state
        .model_registry
        .get_model(&payload.model)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("Unknown model: {}", payload.model),
            )
        })?;

    if !model_metadata.is_local {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot delete cloud models".to_string(),
        ));
    }

    println!("🗑️ Deleting model: {}", payload.model);

    state
        .ollama_provider
        .delete_model(&payload.model)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete model: {}", e),
            )
        })?;

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
