use super::{GenerateResponse, ModelProvider, CircuitBreaker, RateLimiter};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
    #[serde(default)]
    total_tokens: Option<u32>,
}

pub struct GroqProvider {
    client: Client,
    api_key: Option<String>,
    rate_limiter: Arc<RateLimiter>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl GroqProvider {
    #[allow(dead_code)]
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self::with_limits(client, api_key, 60, 5, Duration::from_secs(60))
    }

    pub fn with_limits(
        client: Client,
        api_key: Option<String>,
        requests_per_minute: u32,
        circuit_breaker_threshold: u32,
        circuit_breaker_cooldown: Duration,
    ) -> Self {
        Self {
            client,
            api_key,
            rate_limiter: Arc::new(RateLimiter::new(requests_per_minute)),
            circuit_breaker: Arc::new(CircuitBreaker::new(
                circuit_breaker_threshold,
                circuit_breaker_cooldown,
            )),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }
}

#[async_trait]
impl ModelProvider for GroqProvider {
    async fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse> {
        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return Err(anyhow!(
                "Circuit breaker is open (too many failures). Failures: {}",
                self.circuit_breaker.failures()
            ));
        }

        // Wait for rate limiter
        self.rate_limiter.until_ready().await;

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

        // Track network send time
        let send_start = Instant::now();
        let response = match self
            .client
            .post(format!("{}/chat/completions", GROQ_BASE_URL))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure();
                return Err(anyhow!("Failed to send request to Groq: {}", e));
            }
        };
        let network_send_ms = send_start.elapsed().as_millis();

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            self.circuit_breaker.record_failure();
            return Err(anyhow!(
                "Groq API returned error {}: {}",
                status,
                body
            ));
        }

        // Track network receive time
        let receive_start = Instant::now();
        let groq_response: GroqChatResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure();
                return Err(anyhow!("Failed to parse Groq response: {}", e));
            }
        };
        let network_receive_ms = receive_start.elapsed().as_millis();

        let latency_ms = start.elapsed().as_millis();
        let network_total_ms = network_send_ms + network_receive_ms;

        let content = groq_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("Groq API returned no choices"))?;

        // Extract token information from usage
        let prompt_tokens = groq_response
            .usage
            .as_ref()
            .and_then(|u| u.prompt_tokens);

        let completion_tokens = groq_response
            .usage
            .as_ref()
            .and_then(|u| u.completion_tokens);

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

        // Record success
        self.circuit_breaker.record_success();

        Ok(GenerateResponse {
            content,
            latency_ms,
            network_send_ms: Some(network_send_ms),
            network_receive_ms: Some(network_receive_ms),
            network_total_ms: Some(network_total_ms),
            prompt_tokens,
            completion_tokens,
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
