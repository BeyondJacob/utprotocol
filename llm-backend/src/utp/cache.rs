use super::protocol::{CompressedEmbedding, Precision};
use super::compression::Compressor;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Semantic hash for cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticHash(u64);

impl SemanticHash {
    pub fn from_text(text: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        SemanticHash(hasher.finish())
    }
}

/// Cached entry with compression and metadata
#[derive(Debug, Clone)]
pub struct CachedEntry {
    pub embedding: CompressedEmbedding,
    pub response_text: String,
    pub hit_count: Arc<AtomicU32>,
}

/// Cache metrics for tracking performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub cache_size: usize,
    pub evictions: u64,
    pub total_savings_ms: u64,
}

/// Metrics tracking for the cache
pub struct CacheMetrics {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
        )
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe semantic embedding cache
pub struct EmbeddingCache {
    hot_cache: DashMap<SemanticHash, Arc<CachedEntry>>,
    compressor: Compressor,
    pub metrics: Arc<CacheMetrics>,
    max_entries: usize,
}

impl EmbeddingCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            hot_cache: DashMap::new(),
            compressor: Compressor::new(),
            metrics: Arc::new(CacheMetrics::new()),
            max_entries,
        }
    }

    /// Get a cached entry by semantic hash
    pub fn get(&self, key: SemanticHash) -> Option<(String, CompressedEmbedding)> {
        if let Some(entry) = self.hot_cache.get(&key) {
            // Increment hit count
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            self.metrics.record_hit();

            Some((entry.response_text.clone(), entry.embedding.clone()))
        } else {
            self.metrics.record_miss();
            None
        }
    }

    /// Store a new entry in the cache
    pub fn store(&self, key: SemanticHash, embedding: Vec<f32>, response: String) {
        // Evict if at capacity (simple random eviction for MVP)
        if self.hot_cache.len() >= self.max_entries {
            self.evict_one();
        }

        // Compress embedding to Int8 (default for cache)
        let compressed = self.compressor.compress(&embedding, Precision::Int8);

        let entry = Arc::new(CachedEntry {
            embedding: compressed,
            response_text: response,
            hit_count: Arc::new(AtomicU32::new(1)),
        });

        self.hot_cache.insert(key, entry);
    }

    /// Evict one entry (simple random eviction)
    fn evict_one(&self) {
        // For MVP, just remove the first entry we encounter
        // In production, would implement LRU properly
        if let Some(entry) = self.hot_cache.iter().next() {
            let key = *entry.key();
            drop(entry); // Release the reference
            self.hot_cache.remove(&key);
            self.metrics.record_eviction();
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let (hits, misses, evictions) = self.metrics.get_stats();
        let total = hits + misses;

        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        // Estimate savings: assume each cache hit saves 100ms average API call
        let total_savings_ms = hits * 100;

        CacheStats {
            hits,
            misses,
            hit_rate,
            cache_size: self.hot_cache.len(),
            evictions,
            total_savings_ms,
        }
    }

    /// Clear the cache (useful for testing and cache management)
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.hot_cache.clear();
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new(1000) // Default max 1000 entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_hash() {
        let text1 = "Hello, world!";
        let text2 = "Hello, world!";
        let text3 = "Goodbye, world!";

        let hash1 = SemanticHash::from_text(text1);
        let hash2 = SemanticHash::from_text(text2);
        let hash3 = SemanticHash::from_text(text3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_hit_miss() {
        let cache = EmbeddingCache::new(100);
        let key = SemanticHash::from_text("test query");

        // Miss on first access
        assert!(cache.get(key).is_none());

        // Store entry
        let embedding = vec![1.0; 768];
        cache.store(key, embedding.clone(), "test response".to_string());

        // Hit on second access
        let result = cache.get(key);
        assert!(result.is_some());

        let (response, _compressed) = result.unwrap();
        assert_eq!(response, "test response");

        // Check metrics
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.cache_size, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = EmbeddingCache::new(2); // Small cache for testing

        let key1 = SemanticHash::from_text("query1");
        let key2 = SemanticHash::from_text("query2");
        let key3 = SemanticHash::from_text("query3");

        let embedding = vec![1.0; 768];

        // Fill cache
        cache.store(key1, embedding.clone(), "response1".to_string());
        cache.store(key2, embedding.clone(), "response2".to_string());

        assert_eq!(cache.hot_cache.len(), 2);

        // This should trigger eviction
        cache.store(key3, embedding.clone(), "response3".to_string());

        // Cache should still be at max size
        assert_eq!(cache.hot_cache.len(), 2);

        // At least one eviction should have occurred
        let stats = cache.get_stats();
        assert!(stats.evictions > 0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let cache = EmbeddingCache::new(100);
        let key = SemanticHash::from_text("test");

        let embedding = vec![1.0; 768];

        // 1 miss
        cache.get(key);

        // Store
        cache.store(key, embedding, "response".to_string());

        // 2 hits
        cache.get(key);
        cache.get(key);

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.666).abs() < 0.01); // 2/3 ≈ 0.666
    }
}
