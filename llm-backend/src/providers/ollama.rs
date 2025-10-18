use super::{GenerateResponse, ModelProvider};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponseBody {
    response: String,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaListResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize)]
pub struct OllamaPullRequest {
    pub name: String,
    pub stream: bool,
}

pub struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List all downloaded models from Ollama
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/api/tags", OLLAMA_BASE_URL))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let list = response.json::<OllamaListResponse>().await?;
        Ok(list.models.into_iter().map(|m| m.name).collect())
    }

    /// Pull/download a model
    pub async fn pull_model(&self, model: &str) -> Result<()> {
        let request = OllamaPullRequest {
            name: model.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/pull", OLLAMA_BASE_URL))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama pull API returned error: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Delete a model
    pub async fn delete_model(&self, model: &str) -> Result<()> {
        let response = self
            .client
            .delete(format!("{}/api/delete", OLLAMA_BASE_URL))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama delete API returned error: {}",
                response.status()
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    async fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse> {
        let start = Instant::now();

        let request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/generate", OLLAMA_BASE_URL))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama API returned error: {}",
                response.status()
            ));
        }

        let ollama_response: OllamaGenerateResponseBody = response.json().await?;
        let latency_ms = start.elapsed().as_millis();

        // Calculate tokens per second from Ollama's eval_count
        let total_tokens = ollama_response.eval_count.or(ollama_response.prompt_eval_count);
        let tokens_per_second = if let (Some(tokens), Some(duration_ns)) = (total_tokens, ollama_response.total_duration) {
            if duration_ns > 0 {
                Some((tokens as f64 / duration_ns as f64) * 1_000_000_000.0)
            } else {
                None
            }
        } else {
            None
        };

        Ok(GenerateResponse {
            content: ollama_response.response,
            latency_ms,
            prompt_tokens: None,  // Ollama doesn't provide this separately
            completion_tokens: None,  // Ollama doesn't provide this separately
            total_tokens,
            tokens_per_second,
        })
    }

    async fn is_connected(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", OLLAMA_BASE_URL))
            .send()
            .await
            .is_ok()
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}
