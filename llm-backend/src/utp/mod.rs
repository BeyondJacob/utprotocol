pub mod protocol;
pub mod compression;
pub mod cache;
pub mod middleware;
pub mod metrics;
pub mod embeddings;
pub mod similarity;

// Re-export commonly used types for convenience
pub use protocol::{Precision, UtpMetadata};
pub use cache::EmbeddingCache;
pub use compression::Compressor;
pub use middleware::UtpMiddleware;
pub use embeddings::{EmbeddingProvider, OllamaEmbeddingProvider, MockEmbeddingProvider};
