use utp_agent::MessageBus;
use utp_llm::LlamaClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub message_bus: Arc<MessageBus>,
    pub llama_client: Arc<LlamaClient>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            message_bus: MessageBus::new(),
            llama_client: Arc::new(LlamaClient::new("http://127.0.0.1:8080")),
        }
    }
}