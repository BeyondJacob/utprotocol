pub mod ollama;
pub mod groq;

use async_trait::async_trait;
use anyhow::Result;

/// Response from a model provider's generate call
#[derive(Debug, Clone)]
pub struct GenerateResponse {
    pub content: String,
    pub latency_ms: u128,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub tokens_per_second: Option<f64>,
}

/// Trait for LLM providers (Ollama, Groq, OpenAI, etc.)
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Generate a response from the model
    async fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse>;

    /// Check if the provider is available/connected
    async fn is_connected(&self) -> bool;

    /// Get the provider name
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

/// Pricing information for cloud models (per million tokens)
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
}

/// Model metadata including provider information
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub name: String,
    pub display_name: String,
    pub provider: ProviderType,
    pub is_local: bool,
    pub pricing: Option<ModelPricing>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Ollama,
    Groq,
}

impl ProviderType {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::Ollama => "ollama",
            ProviderType::Groq => "groq",
        }
    }
}

/// Registry of all available models across providers
pub struct ModelRegistry {
    models: Vec<ModelMetadata>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: vec![
                // Local Ollama models
                ModelMetadata {
                    name: "gpt-oss:20b".to_string(),
                    display_name: "GPT-OSS 20B (Local)".to_string(),
                    provider: ProviderType::Ollama,
                    is_local: true,
                    pricing: None,
                },
                ModelMetadata {
                    name: "gpt-oss:120b".to_string(),
                    display_name: "GPT-OSS 120B (Local)".to_string(),
                    provider: ProviderType::Ollama,
                    is_local: true,
                    pricing: None,
                },
                ModelMetadata {
                    name: "llama3.2:1b".to_string(),
                    display_name: "Llama 3.2 1B (Local)".to_string(),
                    provider: ProviderType::Ollama,
                    is_local: true,
                    pricing: None,
                },
                ModelMetadata {
                    name: "llama3.2:3b".to_string(),
                    display_name: "Llama 3.2 3B (Local)".to_string(),
                    provider: ProviderType::Ollama,
                    is_local: true,
                    pricing: None,
                },
                // Cloud Groq models
                ModelMetadata {
                    name: "openai/gpt-oss-20b".to_string(),
                    display_name: "GPT-OSS 20B (Groq)".to_string(),
                    provider: ProviderType::Groq,
                    is_local: false,
                    pricing: Some(ModelPricing {
                        input_per_million: 0.075,
                        output_per_million: 0.30,
                    }),
                },
                ModelMetadata {
                    name: "openai/gpt-oss-120b".to_string(),
                    display_name: "GPT-OSS 120B (Groq)".to_string(),
                    provider: ProviderType::Groq,
                    is_local: false,
                    pricing: Some(ModelPricing {
                        input_per_million: 0.15,
                        output_per_million: 0.60,
                    }),
                },
                ModelMetadata {
                    name: "meta-llama/Llama-3.3-70b-versatile".to_string(),
                    display_name: "Llama 3.3 70B (Groq)".to_string(),
                    provider: ProviderType::Groq,
                    is_local: false,
                    pricing: Some(ModelPricing {
                        input_per_million: 0.59,
                        output_per_million: 0.79,
                    }),
                },
            ],
        }
    }

    pub fn all_models(&self) -> &[ModelMetadata] {
        &self.models
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelMetadata> {
        self.models.iter().find(|m| m.name == name)
    }

    #[allow(dead_code)]
    pub fn models_by_provider(&self, provider: ProviderType) -> Vec<&ModelMetadata> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
