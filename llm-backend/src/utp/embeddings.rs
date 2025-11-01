use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a given text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Get the dimension of embeddings produced by this provider
    fn dimension(&self) -> usize;

    /// Get the provider name
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

/// Ollama embedding provider
pub struct OllamaEmbeddingProvider {
    client: reqwest::Client,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

impl OllamaEmbeddingProvider {
    pub fn new(client: reqwest::Client, model: String) -> Self {
        Self {
            client,
            model,
            base_url: "http://localhost:11434".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(client: reqwest::Client, model: String, base_url: String) -> Self {
        Self {
            client,
            model,
            base_url,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ollama embeddings API returned error: {}",
                response.status()
            ));
        }

        let embedding_response: OllamaEmbeddingResponse = response.json().await?;
        Ok(embedding_response.embedding)
    }

    fn dimension(&self) -> usize {
        // Ollama models typically return 2048-dimensional embeddings
        // This could be made configurable based on the model
        2048
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

/// Mock embedding provider for testing/fallback
pub struct MockEmbeddingProvider {
    dimension: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new(768)
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        use rand::{Rng, SeedableRng};

        // Generate deterministic mock embeddings based on text hash
        // This ensures same text produces same embedding
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let embedding: Vec<f32> = (0..self.dimension)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn name(&self) -> &str {
        "Mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_deterministic() {
        let provider = MockEmbeddingProvider::new(128);

        let text = "Hello, world!";
        let embedding1 = provider.embed(text).await.unwrap();
        let embedding2 = provider.embed(text).await.unwrap();

        assert_eq!(embedding1.len(), 128);
        assert_eq!(embedding1, embedding2, "Same text should produce same embedding");
    }

    #[tokio::test]
    async fn test_mock_embedding_different_texts() {
        let provider = MockEmbeddingProvider::new(128);

        let embedding1 = provider.embed("text 1").await.unwrap();
        let embedding2 = provider.embed("text 2").await.unwrap();

        assert_ne!(embedding1, embedding2, "Different texts should produce different embeddings");
    }
}
