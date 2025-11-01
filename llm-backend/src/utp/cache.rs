use super::protocol::{CompressedEmbedding, Precision};
use super::compression::Compressor;
use super::similarity::cosine_similarity;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

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
    pub full_embedding: Vec<f32>, // NEW: Full-precision embedding for semantic similarity
    pub response_text: String,
    pub hit_count: Arc<AtomicU32>,
    pub last_accessed: Arc<AtomicU64>, // Unix timestamp in seconds
    #[allow(dead_code)]
    pub created_at: u64, // Unix timestamp in seconds
    pub expires_at: u64, // Unix timestamp in seconds
}

impl CachedEntry {
    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }

    /// Update the last accessed timestamp
    pub fn touch(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_accessed.store(now, Ordering::Relaxed);
    }

    /// Get the last accessed timestamp
    pub fn get_last_accessed(&self) -> u64 {
        self.last_accessed.load(Ordering::Relaxed)
    }
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
    ttl: Duration, // Time-to-live for cache entries
}

impl EmbeddingCache {
    pub fn new(max_entries: usize) -> Self {
        Self::with_ttl(max_entries, Duration::from_secs(3600)) // Default 1 hour
    }

    pub fn with_ttl(max_entries: usize, ttl: Duration) -> Self {
        Self {
            hot_cache: DashMap::new(),
            compressor: Compressor::new(),
            metrics: Arc::new(CacheMetrics::new()),
            max_entries,
            ttl,
        }
    }

    /// Get a cached entry by semantic hash
    pub fn get(&self, key: SemanticHash) -> Option<(String, CompressedEmbedding)> {
        if let Some(entry) = self.hot_cache.get(&key) {
            // Check if entry has expired
            if entry.is_expired() {
                drop(entry); // Release the read lock
                self.hot_cache.remove(&key);
                self.metrics.record_miss();
                self.metrics.record_eviction();
                return None;
            }

            // Update access time and increment hit count
            entry.touch();
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            self.metrics.record_hit();

            Some((entry.response_text.clone(), entry.embedding.clone()))
        } else {
            self.metrics.record_miss();
            None
        }
    }

    /// Find semantically similar entries using cosine similarity
    ///
    /// This performs a linear search over all cached embeddings to find
    /// the most similar entry. Returns the entry if similarity >= threshold.
    ///
    /// # Arguments
    /// * `query_embedding` - The query embedding to compare against
    /// * `similarity_threshold` - Minimum cosine similarity (0.0 to 1.0)
    ///
    /// # Returns
    /// Option containing (response_text, compressed_embedding, similarity_score)
    /// if a sufficiently similar entry is found
    pub fn get_similar(
        &self,
        query_embedding: &[f32],
        similarity_threshold: f32,
    ) -> Option<(String, CompressedEmbedding, f32)> {
        let mut best_match: Option<(Arc<CachedEntry>, f32)> = None;

        // Linear search over all cache entries
        // TODO: For production, consider using HNSW or other ANN index
        for entry_ref in self.hot_cache.iter() {
            let entry = entry_ref.value();

            // Skip expired entries
            if entry.is_expired() {
                continue;
            }

            // Compute cosine similarity
            let similarity = cosine_similarity(query_embedding, &entry.full_embedding);

            // Track best match
            if similarity >= similarity_threshold {
                if let Some((_, best_sim)) = &best_match {
                    if similarity > *best_sim {
                        best_match = Some((entry.clone(), similarity));
                    }
                } else {
                    best_match = Some((entry.clone(), similarity));
                }
            }
        }

        // Return best match if found
        if let Some((entry, similarity)) = best_match {
            // Update access time and increment hit count
            entry.touch();
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            self.metrics.record_hit();

            tracing::debug!(
                "Semantic cache hit: similarity={:.4}, threshold={:.4}",
                similarity,
                similarity_threshold
            );

            Some((
                entry.response_text.clone(),
                entry.embedding.clone(),
                similarity,
            ))
        } else {
            self.metrics.record_miss();
            None
        }
    }

    /// Store a new entry in the cache
    pub fn store(&self, key: SemanticHash, embedding: Vec<f32>, response: String) {
        // Evict if at capacity (LRU eviction)
        if self.hot_cache.len() >= self.max_entries {
            self.evict_lru();
        }

        // Calculate timestamps
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + self.ttl.as_secs();

        // Compress embedding to Int8 (default for cache)
        let compressed = self.compressor.compress(&embedding, Precision::Int8);

        let entry = Arc::new(CachedEntry {
            embedding: compressed,
            full_embedding: embedding.clone(), // Store full embedding for semantic search
            response_text: response,
            hit_count: Arc::new(AtomicU32::new(1)),
            last_accessed: Arc::new(AtomicU64::new(now)),
            created_at: now,
            expires_at,
        });

        self.hot_cache.insert(key, entry);
    }

    /// Evict the least recently used entry
    fn evict_lru(&self) {
        // Find the entry with the oldest last_accessed timestamp
        let mut oldest_key: Option<SemanticHash> = None;
        let mut oldest_time = u64::MAX;

        for entry in self.hot_cache.iter() {
            let last_accessed = entry.value().get_last_accessed();
            if last_accessed < oldest_time {
                oldest_time = last_accessed;
                oldest_key = Some(*entry.key());
            }
        }

        // Remove the oldest entry
        if let Some(key) = oldest_key {
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
        Self::new(1000) // Default max 1000 entries with 1 hour TTL
    }
}

#[cfg(test)]
impl EmbeddingCache {
    /// Create cache with short TTL for testing
    pub fn with_short_ttl(max_entries: usize) -> Self {
        Self::with_ttl(max_entries, Duration::from_secs(1))
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
    fn test_semantic_similarity_search() {
        let cache = EmbeddingCache::new(100);

        // Store a few entries with known embeddings
        let embedding1 = vec![1.0, 0.0, 0.0]; // Unit vector in x direction
        let embedding2 = vec![0.9, 0.1, 0.0]; // Very similar to embedding1
        let embedding3 = vec![0.0, 1.0, 0.0]; // Orthogonal to embedding1

        cache.store(
            SemanticHash::from_text("query1"),
            embedding1.clone(),
            "response1".to_string(),
        );
        cache.store(
            SemanticHash::from_text("query2"),
            embedding2.clone(),
            "response2".to_string(),
        );
        cache.store(
            SemanticHash::from_text("query3"),
            embedding3.clone(),
            "response3".to_string(),
        );

        // Query with embedding very similar to embedding1
        let query = vec![0.95, 0.05, 0.0];

        // Should find embedding1 or embedding2 with high similarity
        let result = cache.get_similar(&query, 0.95);
        assert!(result.is_some());

        let (response, _compressed, similarity) = result.unwrap();
        assert!(similarity >= 0.95);
        // Should be response1 or response2
        assert!(response == "response1" || response == "response2");

        // Query with embedding orthogonal to all stored embeddings
        let orthogonal_query = vec![0.0, 0.0, 1.0];
        let result = cache.get_similar(&orthogonal_query, 0.9);
        assert!(result.is_none()); // Should not find anything with similarity >= 0.9
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
