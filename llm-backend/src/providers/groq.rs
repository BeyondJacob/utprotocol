use super::{GenerateResponse, ModelProvider};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

#[derive(Debug, Serialize)]
struct GroqChatRequest {
    model: String,
    messages: Vec<GroqMessage>,
}

#[derive(Debug, Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqChatResponse {
    choices: Vec<GroqChoice>,
    #[serde(default)]
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessageContent,
}

#[derive(Debug, Deserialize)]
struct GroqMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    #[serde(default)]
    total_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
}

pub struct GroqProvider {
    client: Client,
    api_key: Option<String>,
}

impl GroqProvider {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self { client, api_key }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }
}

#[async_trait]
impl ModelProvider for GroqProvider {
    async fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse> {
        let start = Instant::now();

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("Groq API key not configured"))?;

        let request = GroqChatRequest {
            model: model.to_string(),
            messages: vec![GroqMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", GROQ_BASE_URL))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Groq API returned error {}: {}",
                status,
                body
            ));
        }

        let groq_response: GroqChatResponse = response.json().await?;
        let latency_ms = start.elapsed().as_millis();

        let content = groq_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("Groq API returned no choices"))?;

        // Extract token information from usage
        let total_tokens = groq_response
            .usage
            .as_ref()
            .and_then(|u| u.total_tokens);

        let tokens_per_second = if let Some(tokens) = total_tokens {
            if latency_ms > 0 {
                Some((tokens as f64 / latency_ms as f64) * 1000.0)
            } else {
                None
            }
        } else {
            None
        };

        Ok(GenerateResponse {
            content,
            latency_ms,
            total_tokens,
            tokens_per_second,
        })
    }

    async fn is_connected(&self) -> bool {
        // For cloud providers, we consider them "connected" if API key is configured
        // We could optionally make a test API call here
        self.is_configured()
    }

    fn name(&self) -> &str {
        "Groq"
    }
}
