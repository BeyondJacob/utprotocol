use axum::{
    extract::{DefaultBodyLimit, Path, State},
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
mod utp;
mod agents;
mod config;
mod pdf_processor;
mod rag_handlers;

use db::{
    models::{ConversationWithMessages, CreateConversationRequest},
    repository::{ConversationRepository, MessageRepository},
    vector_repository::VectorRepository,
};
use rag_handlers::{
    list_documents_handler, get_document_handler, delete_document_handler,
    upload_document_handler, rag_query_handler, rag_statistics_handler,
};
use providers::{
    groq::GroqProvider,
    ollama::OllamaProvider,
    ModelProvider, ModelRegistry, ProviderType,
};
use utp::{
    EmbeddingCache,
    Compressor,
    UtpMiddleware,
    UtpMetadata,
};
use agents::{
    AgentRegistry,
    AgentExecutor,
    FlowExecutor,
    CreateAgentRequest,
    UpdateAgentRequest,
    CreateFlowRequest,
    CreateEdgeRequest,
    ExecuteAgentRequest,
    ExecuteFlowRequest,
};

// Shared application state
#[derive(Clone)]
struct AppState {
    active_model: Arc<RwLock<String>>,
    ollama_provider: Arc<OllamaProvider>,
    groq_provider: Arc<GroqProvider>,
    model_registry: Arc<ModelRegistry>,
    db_pool: PgPool,
    utp_middleware: Arc<UtpMiddleware>,
    agent_registry: Arc<AgentRegistry>,
    agent_executor: Arc<AgentExecutor>,
    flow_executor: Arc<FlowExecutor>,
    vector_repository: Arc<VectorRepository>,
    embedding_provider: Arc<dyn utp::EmbeddingProvider>,
}

// Request/Response types
#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    message: String,
    conversation_id: Option<i32>,
    use_utp: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    model: String,
    latency_ms: u128,
    network_send_ms: Option<u128>,
    network_receive_ms: Option<u128>,
    network_total_ms: Option<u128>,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    tokens_per_second: Option<f64>,
    total_tokens: Option<u32>,
    utp_metadata: Option<UtpMetadata>,
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
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = config::Config::load();
    tracing::info!("Configuration loaded");

    // Load environment variables from .env file
    dotenvy::from_filename("../.env").ok();

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://jacobowens@localhost/utprotocol".to_string());

    let db_pool = db::init_db(&database_url)
        .await
        .expect("Failed to initialize database");

    tracing::info!("Database connected and initialized");

    // Initialize HTTP client with timeout
    let http_client = reqwest::Client::builder()
        .timeout(config.provider_timeout("groq"))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to create HTTP client");

    // Initialize providers with configuration
    let groq_api_key = std::env::var("GROQ_API_KEY").ok();
    let ollama_provider = Arc::new(OllamaProvider::new(http_client.clone()));
    let groq_provider = Arc::new(GroqProvider::with_limits(
        http_client.clone(),
        groq_api_key.clone(),
        config.providers.groq.rate_limit_per_minute,
        config.providers.groq.circuit_breaker_threshold,
        std::time::Duration::from_secs(config.providers.groq.circuit_breaker_cooldown_seconds),
    ));
    let model_registry = Arc::new(ModelRegistry::new());

    // Check provider status
    if ollama_provider.is_connected().await {
        tracing::info!("Ollama provider connected");
    } else {
        tracing::warn!("Ollama provider not connected");
    }

    if groq_provider.is_connected().await {
        tracing::info!("Groq provider configured");
    } else {
        tracing::warn!("Groq provider not configured (GROQ_API_KEY missing)");
    }

    // Initialize UTP middleware with configuration
    let cache = Arc::new(EmbeddingCache::with_ttl(
        config.cache.max_entries,
        config.cache_ttl(),
    ));
    let compressor = Compressor::new();

    // Initialize embedding provider (Ollama with gpt-oss:20b for embeddings)
    // Falls back to mock embeddings if Ollama is not available
    let embedding_provider: Arc<dyn utp::EmbeddingProvider> = if ollama_provider.is_connected().await {
        tracing::info!("Using Ollama for embeddings (model: nomic-embed-text)");
        Arc::new(utp::OllamaEmbeddingProvider::new(
            http_client.clone(),
            "nomic-embed-text".to_string(),
        ))
    } else {
        tracing::warn!("Ollama not connected, using mock embeddings");
        Arc::new(utp::MockEmbeddingProvider::default())
    };

    let utp_middleware = Arc::new(UtpMiddleware::with_similarity_threshold(
        cache,
        compressor,
        embedding_provider.clone(),
        config.cache.similarity_threshold,
    ));

    tracing::info!(
        "UTP middleware initialized (cache size: {}, TTL: {}s, similarity_threshold: {:.2}, compression: Int8)",
        config.cache.max_entries,
        config.cache.ttl_seconds,
        config.cache.similarity_threshold
    );

    // Initialize agent system
    let agent_registry = Arc::new(AgentRegistry::new(db_pool.clone()));
    let agent_executor = Arc::new(AgentExecutor::new(agent_registry.clone()));
    let flow_executor = Arc::new(FlowExecutor::new(agent_registry.clone(), agent_executor.clone()));

    tracing::info!("Agent system initialized");

    // Initialize vector repository for RAG
    let vector_repository = Arc::new(VectorRepository::new(db_pool.clone()));
    tracing::info!("Vector repository initialized");

    // Initialize state
    let state = AppState {
        active_model: Arc::new(RwLock::new("gpt-oss:20b".to_string())),
        ollama_provider,
        groq_provider,
        model_registry,
        db_pool,
        utp_middleware,
        agent_registry,
        agent_executor,
        flow_executor,
        vector_repository,
        embedding_provider,
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
        // UTP stats route
        .route("/utp/stats", get(utp_stats_handler))
        // RAG/Vector routes
        .route("/documents", get(list_documents_handler))
        .route("/documents/upload", post(upload_document_handler))
        .route("/documents/:id", get(get_document_handler))
        .route("/documents/:id", delete(delete_document_handler))
        .route("/rag/query", post(rag_query_handler))
        .route("/rag/statistics", get(rag_statistics_handler))
        // Agent routes
        .route("/agents", get(get_agents_handler))
        .route("/agents", post(create_agent_handler))
        .route("/agents/:id", get(get_agent_handler))
        .route("/agents/:id", post(update_agent_handler))
        .route("/agents/:id", delete(delete_agent_handler))
        .route("/agents/:id/execute", post(execute_agent_handler))
        // Flow routes
        .route("/flows", get(get_flows_handler))
        .route("/flows", post(create_flow_handler))
        .route("/flows/:id", get(get_flow_handler))
        .route("/flows/:id", delete(delete_flow_handler))
        .route("/flows/:id/execute", post(execute_flow_handler))
        .route("/flows/:flow_id/agents/:agent_id", post(add_agent_to_flow_handler))
        // Edge routes
        .route("/edges", post(create_edge_handler))
        .route("/flows/:id/edges", get(get_flow_edges_handler))
        .with_state(state)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))  // 50MB limit for PDF uploads
        .layer(cors);

    // Start server
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind server");

    tracing::info!("Server running on http://{}", bind_addr);

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

// Chat handler (UTP-enabled)
async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Determine if UTP should be used
    let use_utp = if let Some(conversation_id) = payload.conversation_id {
        // Get UTP setting from conversation
        let conv_repo = ConversationRepository::new(state.db_pool.clone());
        let conversation = conv_repo
            .get_conversation(conversation_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to fetch conversation: {}", e),
                )
            })?
            .ok_or((StatusCode::NOT_FOUND, "Conversation not found".to_string()))?;

        conversation.utp_enabled
    } else {
        // Use explicit flag from request, default to false
        payload.use_utp.unwrap_or(false)
    };

    // Save user message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());
        message_repo
            .create_message(
                conversation_id,
                "user",
                &payload.message,
                Some(&payload.model),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
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

    // Get the appropriate provider
    let provider: Arc<dyn ModelProvider> = match model_metadata.provider {
        ProviderType::Ollama => state.ollama_provider.clone(),
        ProviderType::Groq => state.groq_provider.clone(),
    };

    // Route through UTP middleware
    let utp_response = state
        .utp_middleware
        .generate_with_utp(provider, &payload.model, &payload.message, use_utp)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate response: {}", e),
            )
        })?;

    let latency_ms = utp_response.response.latency_ms;

    // Save assistant message if conversation_id is provided
    if let Some(conversation_id) = payload.conversation_id {
        let message_repo = MessageRepository::new(state.db_pool.clone());

        // Serialize UTP metadata to JSON
        let utp_metadata_json = serde_json::to_value(&utp_response.metadata)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to serialize UTP metadata: {}", e),
                )
            })?;

        message_repo
            .create_message(
                conversation_id,
                "assistant",
                &utp_response.response.content,
                Some(&payload.model),
                Some(latency_ms as i32),
                utp_response.response.prompt_tokens.map(|t| t as i32),
                utp_response.response.completion_tokens.map(|t| t as i32),
                utp_response.response.tokens_per_second,
                utp_response.response.total_tokens.map(|t| t as i32),
                Some(utp_metadata_json),
                utp_response.response.network_send_ms.map(|t| t as i32),
                utp_response.response.network_receive_ms.map(|t| t as i32),
                utp_response.response.network_total_ms.map(|t| t as i32),
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
        response: utp_response.response.content,
        model: payload.model,
        latency_ms,
        network_send_ms: utp_response.response.network_send_ms,
        network_receive_ms: utp_response.response.network_receive_ms,
        network_total_ms: utp_response.response.network_total_ms,
        prompt_tokens: utp_response.response.prompt_tokens,
        completion_tokens: utp_response.response.completion_tokens,
        tokens_per_second: utp_response.response.tokens_per_second,
        total_tokens: utp_response.response.total_tokens,
        utp_metadata: Some(utp_response.metadata),
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
        .create_conversation(&payload.title, &payload.model, payload.utp_enabled)
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

// UTP stats handler
async fn utp_stats_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let stats = state.utp_middleware.get_comparison_stats();
    Json(stats)
}

// Agent handlers
async fn get_agents_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = state.agent_registry.get_all_agents().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch agents: {}", e),
        )
    })?;

    Ok(Json(agents))
}

async fn get_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = state
        .agent_registry
        .get_agent(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch agent: {}", e),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    Ok(Json(agent))
}

async fn create_agent_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = state
        .agent_registry
        .create_agent(payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create agent: {}", e),
            )
        })?;

    Ok(Json(agent))
}

async fn update_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAgentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = state
        .agent_registry
        .update_agent(id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update agent: {}", e),
            )
        })?;

    Ok(Json(agent))
}

async fn delete_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.agent_registry.delete_agent(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete agent: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Agent deleted"
    })))
}

async fn execute_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<ExecuteAgentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get agent to determine which provider to use
    let agent = state
        .agent_registry
        .get_agent(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch agent: {}", e),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    // Determine provider based on model
    let model = agent.model.as_ref().ok_or((
        StatusCode::BAD_REQUEST,
        "Agent has no model configured".to_string(),
    ))?;

    let model_metadata = state
        .model_registry
        .get_model(model)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("Unknown model: {}", model),
            )
        })?;

    let provider: Arc<dyn ModelProvider> = match model_metadata.provider {
        ProviderType::Ollama => state.ollama_provider.clone(),
        ProviderType::Groq => state.groq_provider.clone(),
    };

    // Execute agent
    let response = state
        .agent_executor
        .execute_agent(id, payload.input, provider)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to execute agent: {}", e),
            )
        })?;

    Ok(Json(response))
}

// Flow handlers
async fn get_flows_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let flows = state.agent_registry.get_all_flows().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch flows: {}", e),
        )
    })?;

    Ok(Json(flows))
}

async fn get_flow_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let flow = state
        .agent_registry
        .get_flow_with_agents(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch flow: {}", e),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Flow not found".to_string()))?;

    Ok(Json(flow))
}

async fn create_flow_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateFlowRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let flow = state
        .agent_registry
        .create_flow(payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create flow: {}", e),
            )
        })?;

    Ok(Json(flow))
}

async fn delete_flow_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.agent_registry.delete_flow(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete flow: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Flow deleted"
    })))
}

async fn execute_flow_handler(
    State(state): State<AppState>,
    Path(flow_id): Path<i32>,
    Json(payload): Json<ExecuteFlowRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Provider resolver closure
    let model_registry = state.model_registry.clone();
    let ollama = state.ollama_provider.clone();
    let groq = state.groq_provider.clone();

    let provider_resolver = move |model: &str| -> Option<Arc<dyn ModelProvider>> {
        let model_metadata = model_registry.get_model(model)?;
        match model_metadata.provider {
            ProviderType::Ollama => Some(ollama.clone()),
            ProviderType::Groq => Some(groq.clone()),
        }
    };

    // Execute flow
    let result = state
        .flow_executor
        .execute_flow(flow_id, payload.input, payload.start_node_id, provider_resolver)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to execute flow: {}", e),
            )
        })?;

    Ok(Json(result))
}

async fn add_agent_to_flow_handler(
    State(state): State<AppState>,
    Path((flow_id, agent_id)): Path<(i32, i32)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .agent_registry
        .add_agent_to_flow(flow_id, agent_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to add agent to flow: {}", e),
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Agent added to flow"
    })))
}

// Edge handlers
async fn create_edge_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateEdgeRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let edge = state
        .agent_registry
        .create_edge(payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create edge: {}", e),
            )
        })?;

    Ok(Json(edge))
}

async fn get_flow_edges_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let edges = state
        .agent_registry
        .get_flow_edges(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch edges: {}", e),
            )
        })?;

    Ok(Json(edges))
}
