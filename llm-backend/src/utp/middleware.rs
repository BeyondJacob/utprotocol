use super::cache::{CacheStats, EmbeddingCache, SemanticHash};
use super::compression::Compressor;
use super::embeddings::EmbeddingProvider;
use super::metrics::{MetricsSnapshot, UtpMetrics};
use super::protocol::{Precision, UtpMetadata};
use crate::providers::{GenerateResponse, ModelProvider};
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
    embedding_provider: Arc<dyn EmbeddingProvider>,
    pub metrics: Arc<UtpMetrics>,
    similarity_threshold: f32, // NEW: Threshold for semantic similarity cache hits
}

impl UtpMiddleware {
    pub fn new(
        cache: Arc<EmbeddingCache>,
        compressor: Compressor,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self::with_similarity_threshold(cache, compressor, embedding_provider, 0.92)
    }

    pub fn with_similarity_threshold(
        cache: Arc<EmbeddingCache>,
        compressor: Compressor,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        similarity_threshold: f32,
    ) -> Self {
        Self {
            cache,
            compressor,
            embedding_provider,
            metrics: Arc::new(UtpMetrics::new()),
            similarity_threshold,
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

            // Use actual embedding dimension from provider
            let embedding_dim = self.embedding_provider.dimension();
            let estimated_size = embedding_dim * 4; // f32 = 4 bytes per dimension

            // Record traditional metrics
            self.metrics.record_traditional(latency_us, estimated_size);

            let metadata = UtpMetadata::traditional(latency_us, estimated_size);

            return Ok(UtpGenerateResponse { response, metadata });
        }

        // UTP path with caching and compression
        let semantic_hash = SemanticHash::from_text(prompt);

        // First, try exact match (fast path)
        let cache_start = Instant::now();
        let exact_match = self.cache.get(semantic_hash);

        if let Some((cached_response, cached_embedding)) = exact_match {
            let latency_us = start.elapsed().as_micros() as u64;
            let cache_lookup_time_us = cache_start.elapsed().as_micros() as u64;

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

            tracing::debug!("Exact cache hit for query");

            let metadata = UtpMetadata::utp(
                true, // cache_hit
                compression_ratio,
                latency_us,
                original_size,
                compressed_size,
                cached_embedding.precision,
                0, // no compression needed on cache hit
                cache_lookup_time_us,
                0, // no LLM call on cache hit
            );

            // Create a mock GenerateResponse from cache
            let response = GenerateResponse {
                content: cached_response,
                latency_ms: (latency_us / 1000) as u128,
                network_send_ms: None,
                network_receive_ms: None,
                network_total_ms: None,
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
                tokens_per_second: None,
            };

            return Ok(UtpGenerateResponse { response, metadata });
        }

        // Exact match failed - generate embedding for semantic search
        let embedding_start = Instant::now();
        let query_embedding = match self.embedding_provider.embed(prompt).await {
            Ok(emb) => emb,
            Err(e) => {
                tracing::warn!("Failed to generate embedding: {}, using fallback", e);
                self.generate_fallback_embedding(prompt)
            }
        };
        let embedding_time_us = embedding_start.elapsed().as_micros() as u64;

        // Try semantic similarity search
        let semantic_result = self.cache.get_similar(&query_embedding, self.similarity_threshold);
        let cache_lookup_time_us = cache_start.elapsed().as_micros() as u64;

        if let Some((cached_response, cached_embedding, similarity)) = semantic_result {
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

            tracing::info!(
                "Semantic cache hit: similarity={:.4}, threshold={:.4}, saved LLM call",
                similarity,
                self.similarity_threshold
            );

            let metadata = UtpMetadata::utp(
                true, // cache_hit
                compression_ratio,
                latency_us,
                original_size,
                compressed_size,
                cached_embedding.precision,
                0, // no compression on cache hit
                cache_lookup_time_us,
                embedding_time_us, // embedding was generated for semantic search
            );

            // Create a mock GenerateResponse from cache
            let response = GenerateResponse {
                content: cached_response,
                latency_ms: (latency_us / 1000) as u128,
                network_send_ms: None,
                network_receive_ms: None,
                network_total_ms: None,
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
                tokens_per_second: None,
            };

            return Ok(UtpGenerateResponse { response, metadata });
        }

        // Cache miss - generate new response
        // Note: We already generated the embedding above for semantic search,
        // so we reuse it here (query_embedding)
        let llm_start = Instant::now();
        let response = provider.generate(model, prompt).await?;
        let llm_time_us = llm_start.elapsed().as_micros() as u64;

        // Compress embedding (default to Int8 for cache)
        let compress_start = Instant::now();
        let compressed = self.compressor.compress(&query_embedding, Precision::Int8);
        let compression_time_us = compress_start.elapsed().as_micros() as u64;

        let original_size = query_embedding.len() * 4; // f32 = 4 bytes
        let compressed_size = self
            .compressor
            .get_compressed_size(compressed.dimension, compressed.precision);
        let compression_ratio = self
            .compressor
            .calculate_compression_ratio(original_size, compressed_size);

        // Store in cache
        self.cache
            .store(semantic_hash, query_embedding, response.content.clone());

        tracing::debug!(
            "UTP: Generated embedding ({} dims) in {}μs, compressed to {} bytes ({}x compression) in {}μs",
            compressed.dimension,
            embedding_time_us,
            compressed_size,
            compression_ratio,
            compression_time_us
        );

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

    /// Fallback embedding generation when real embedding provider fails
    /// Uses deterministic mock embeddings based on text hash for consistent caching
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        use rand::{Rng, SeedableRng};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Generate deterministic embeddings based on text hash
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let dimension = self.embedding_provider.dimension();

        (0..dimension)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect()
    }
}

impl Default for UtpMiddleware {
    fn default() -> Self {
        use super::embeddings::MockEmbeddingProvider;
        Self::new(
            Arc::new(EmbeddingCache::default()),
            Compressor::default(),
            Arc::new(MockEmbeddingProvider::default()),
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
                network_send_ms: Some(10),
                network_receive_ms: Some(5),
                network_total_ms: Some(15),
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
