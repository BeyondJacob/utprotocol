use super::cache::{CacheStats, EmbeddingCache, SemanticHash};
use super::compression::Compressor;
use super::metrics::{MetricsSnapshot, UtpMetrics};
use super::protocol::{Precision, UtpMetadata};
use crate::providers::{GenerateResponse, ModelProvider};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Response from UTP-enhanced generation
#[derive(Debug, Clone)]
pub struct UtpGenerateResponse {
    pub response: GenerateResponse,
    pub metadata: UtpMetadata,
}

/// Comparison statistics between traditional and UTP approaches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonStats {
    pub cache_stats: CacheStats,
    pub performance_metrics: MetricsSnapshot,
}

/// UTP Middleware wrapping model providers with compression and caching
pub struct UtpMiddleware {
    cache: Arc<EmbeddingCache>,
    compressor: Compressor,
    pub metrics: Arc<UtpMetrics>,
}

impl UtpMiddleware {
    pub fn new(cache: Arc<EmbeddingCache>, compressor: Compressor) -> Self {
        Self {
            cache,
            compressor,
            metrics: Arc::new(UtpMetrics::new()),
        }
    }

    /// Generate response with optional UTP optimizations
    pub async fn generate_with_utp(
        &self,
        provider: Arc<dyn ModelProvider>,
        model: &str,
        prompt: &str,
        use_utp: bool,
    ) -> anyhow::Result<UtpGenerateResponse> {
        let start = Instant::now();

        if !use_utp {
            // Traditional path - no optimization
            let response = provider.generate(model, prompt).await?;
            let latency_us = start.elapsed().as_micros() as u64;

            // Estimate size for traditional approach based on prompt length
            let embedding_dim = self.estimate_embedding_dimension(prompt);
            let estimated_size = embedding_dim * 4; // f32 = 4 bytes per dimension

            // Record traditional metrics
            self.metrics.record_traditional(latency_us, estimated_size);

            let metadata = UtpMetadata::traditional(latency_us, estimated_size);

            return Ok(UtpGenerateResponse { response, metadata });
        }

        // UTP path with caching and compression
        let semantic_hash = SemanticHash::from_text(prompt);

        // Estimate embedding dimension based on prompt length
        let embedding_dim = self.estimate_embedding_dimension(prompt);

        // Compression timing (even for cache hit, we measure hypothetical compression)
        let compress_start = Instant::now();
        let mock_embedding = self.generate_mock_embedding(embedding_dim);
        let _compressed_test = self.compressor.compress(&mock_embedding, Precision::Int8);
        let compression_time_us = compress_start.elapsed().as_micros() as u64;

        // Cache lookup timing
        let cache_start = Instant::now();
        let cache_result = self.cache.get(semantic_hash);
        let cache_lookup_time_us = cache_start.elapsed().as_micros() as u64;

        // Check cache
        if let Some((cached_response, cached_embedding)) = cache_result {
            let latency_us = start.elapsed().as_micros() as u64;

            // Calculate compression metrics from cached embedding
            let original_size = cached_embedding.dimension as usize * 4; // f32 = 4 bytes
            let compressed_size = self
                .compressor
                .get_compressed_size(cached_embedding.dimension, cached_embedding.precision);
            let compression_ratio =
                self.compressor
                    .calculate_compression_ratio(original_size, compressed_size);

            // Record UTP metrics
            self.metrics.record_utp(latency_us, compressed_size);

            let metadata = UtpMetadata::utp(
                true, // cache_hit
                compression_ratio,
                latency_us,
                original_size,
                compressed_size,
                cached_embedding.precision,
                compression_time_us,
                cache_lookup_time_us,
                0, // no LLM call on cache hit
            );

            // Create a mock GenerateResponse from cache
            let response = GenerateResponse {
                content: cached_response,
                latency_ms: (latency_us / 1000) as u128,
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
                tokens_per_second: None,
            };

            return Ok(UtpGenerateResponse { response, metadata });
        }

        // Cache miss - generate new response
        let llm_start = Instant::now();
        let response = provider.generate(model, prompt).await?;
        let llm_time_us = llm_start.elapsed().as_micros() as u64;

        // Compress embedding (default to Int8 for cache)
        // Note: mock_embedding already generated above for timing
        let compressed = self.compressor.compress(&mock_embedding, Precision::Int8);
        let original_size = embedding_dim * 4; // f32 = 4 bytes
        let compressed_size = self
            .compressor
            .get_compressed_size(compressed.dimension, compressed.precision);
        let compression_ratio = self
            .compressor
            .calculate_compression_ratio(original_size, compressed_size);

        // Store in cache
        self.cache
            .store(semantic_hash, mock_embedding, response.content.clone());

        let latency_us = start.elapsed().as_micros() as u64;

        // Record UTP metrics
        self.metrics.record_utp(latency_us, compressed_size);

        let metadata = UtpMetadata::utp(
            false, // cache_hit
            compression_ratio,
            latency_us,
            original_size,
            compressed_size,
            Precision::Int8,
            compression_time_us,
            cache_lookup_time_us,
            llm_time_us,
        );

        Ok(UtpGenerateResponse { response, metadata })
    }

    /// Get comparison statistics between traditional and UTP
    pub fn get_comparison_stats(&self) -> ComparisonStats {
        ComparisonStats {
            cache_stats: self.cache.get_stats(),
            performance_metrics: self.metrics.get_stats(),
        }
    }

    /// Generate mock embedding for MVP (random normal distribution)
    /// In production, this would be replaced with actual model embeddings
    fn generate_mock_embedding(&self, dimension: usize) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        (0..dimension)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect()
    }

    /// Estimate embedding dimension based on prompt length
    /// Rough approximation: 1 token ≈ 4 chars, models use ~1-2 dimensions per token
    /// Common models: 768-dim (small), 1024-dim (medium), 1536-dim (large)
    fn estimate_embedding_dimension(&self, prompt: &str) -> usize {
        // Estimate token count (rough: 1 token ≈ 4 characters)
        let estimated_tokens = (prompt.len() / 4).max(1);

        // Base dimension per model (simulating different model sizes)
        // We'll use a base of 768 for smaller prompts, scaling up for longer ones
        let base_dim = 768;

        // Scale dimension based on token count
        // Small prompts (~10 tokens): 768 dim
        // Medium prompts (~50 tokens): 1024 dim
        // Large prompts (~200+ tokens): 1536-2048 dim
        match estimated_tokens {
            1..=20 => base_dim,                          // 768
            21..=100 => base_dim + (estimated_tokens * 2), // 768-968
            _ => (base_dim * 2).min(2048),              // 1536, capped at 2048
        }
    }
}

impl Default for UtpMiddleware {
    fn default() -> Self {
        Self::new(
            Arc::new(EmbeddingCache::default()),
            Compressor::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::GenerateResponse;
    use async_trait::async_trait;

    // Mock provider for testing
    struct MockProvider {
        response: String,
    }

    #[async_trait]
    impl ModelProvider for MockProvider {
        async fn generate(&self, _model: &str, _prompt: &str) -> anyhow::Result<GenerateResponse> {
            Ok(GenerateResponse {
                content: self.response.clone(),
                latency_ms: 100,
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                total_tokens: Some(30),
                tokens_per_second: Some(200.0),
            })
        }

        async fn is_connected(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_traditional_path() {
        let middleware = UtpMiddleware::default();
        let provider = Arc::new(MockProvider {
            response: "Test response".to_string(),
        });

        let result = middleware
            .generate_with_utp(provider, "test-model", "test prompt", false)
            .await
            .unwrap();

        assert_eq!(result.metadata.used_utp, false);
        assert_eq!(result.metadata.cache_hit, false);
        assert_eq!(result.metadata.compression_ratio, 1.0);
        assert_eq!(result.response.content, "Test response");
    }

    #[tokio::test]
    async fn test_utp_path_miss() {
        let middleware = UtpMiddleware::default();
        let provider = Arc::new(MockProvider {
            response: "Test response".to_string(),
        });

        let result = middleware
            .generate_with_utp(provider, "test-model", "test prompt", true)
            .await
            .unwrap();

        assert_eq!(result.metadata.used_utp, true);
        assert_eq!(result.metadata.cache_hit, false);
        assert!(result.metadata.compression_ratio > 1.0); // Should have compression
        assert_eq!(result.response.content, "Test response");
    }

    #[tokio::test]
    async fn test_utp_path_hit() {
        let middleware = UtpMiddleware::default();
        let provider = Arc::new(MockProvider {
            response: "Test response".to_string(),
        });

        // First call - cache miss
        let _result1 = middleware
            .generate_with_utp(provider.clone(), "test-model", "test prompt", true)
            .await
            .unwrap();

        // Second call - cache hit
        let result2 = middleware
            .generate_with_utp(provider, "test-model", "test prompt", true)
            .await
            .unwrap();

        assert_eq!(result2.metadata.used_utp, true);
        assert_eq!(result2.metadata.cache_hit, true);
        assert!(result2.metadata.compression_ratio > 1.0);
        assert_eq!(result2.response.content, "Test response");
    }

    #[tokio::test]
    async fn test_comparison_stats() {
        let middleware = UtpMiddleware::default();
        let provider = Arc::new(MockProvider {
            response: "Test".to_string(),
        });

        // Make some traditional requests
        middleware
            .generate_with_utp(provider.clone(), "test", "prompt1", false)
            .await
            .unwrap();

        // Make some UTP requests
        middleware
            .generate_with_utp(provider.clone(), "test", "prompt2", true)
            .await
            .unwrap();

        let stats = middleware.get_comparison_stats();

        assert!(stats.performance_metrics.total_requests >= 2);
    }
}
