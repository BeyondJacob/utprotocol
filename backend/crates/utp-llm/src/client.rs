use crate::types::{ChatRequest, ChatResponse};
use anyhow::Result;
use reqwest::Client;

pub struct LlamaClient {
    client: Client,
    base_url: String,
}

impl LlamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let response = self.client.post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await?;
        let response = response.json::<ChatResponse>().await?;
        Ok(response)
    }
}