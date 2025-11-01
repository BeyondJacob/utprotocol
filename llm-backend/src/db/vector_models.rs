/// Vector storage models for RAG comparison (Traditional vs UTP)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Uploaded document (PDF, text, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub id: i32,
    pub title: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size_bytes: i32,
    pub total_chunks: i32,
    pub upload_date: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Create document request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub file_name: String,
    pub file_type: String,
}

/// Document chunk with dual embeddings (F32 + Int8)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentChunk {
    pub id: i32,
    pub document_id: i32,
    pub chunk_index: i32,
    pub content: String,
    pub chunk_size: i32,

    // Traditional F32 embedding
    pub embedding_f32: Option<Vec<f32>>,
    pub embedding_f32_dimension: Option<i32>,
    pub embedding_f32_size_bytes: Option<i32>,

    // UTP compressed embedding (stored as binary)
    pub embedding_int8: Option<Vec<u8>>,
    pub embedding_int8_dimension: Option<i32>,
    pub embedding_int8_size_bytes: Option<i32>,
    pub compression_ratio: Option<f32>,

    pub created_at: DateTime<Utc>,
}

/// Chunk creation request
#[derive(Debug, Clone)]
pub struct CreateChunkRequest {
    pub document_id: i32,
    pub chunk_index: i32,
    pub content: String,
    pub embedding_f32: Vec<f32>,
    pub embedding_int8: Vec<u8>,
}

/// RAG query result comparison
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RagQuery {
    pub id: i32,
    pub query_text: String,
    pub query_embedding_f32: Option<Vec<f32>>,

    // Traditional metrics
    pub traditional_latency_ms: Option<i32>,
    pub traditional_top_k_chunks: Option<Vec<i32>>,
    pub traditional_total_size_bytes: Option<i32>,

    // UTP metrics
    pub utp_latency_ms: Option<i32>,
    pub utp_top_k_chunks: Option<Vec<i32>>,
    pub utp_total_size_bytes: Option<i32>,
    pub utp_cache_hit: Option<bool>,
    pub utp_similarity_score: Option<f32>,

    // Comparison metrics
    pub speedup_factor: Option<f32>,
    pub size_reduction_percent: Option<f32>,
    pub retrieval_overlap_percent: Option<f32>,

    pub created_at: DateTime<Utc>,
}

/// RAG query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQueryRequest {
    pub query: String,
    pub top_k: Option<i32>,  // Number of chunks to retrieve
    pub use_utp: bool,       // Compare traditional vs UTP
}

/// RAG query response (single approach)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQueryResponse {
    pub query: String,
    pub chunks: Vec<RetrievedChunk>,
    pub latency_ms: u128,
    pub total_size_bytes: usize,
    pub approach: String,  // "traditional" or "utp"
    pub cache_hit: Option<bool>,
    pub similarity_score: Option<f32>,
}

/// RAG comparison response (both approaches)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagComparisonResponse {
    pub query: String,
    pub traditional: RagQueryResponse,
    pub utp: RagQueryResponse,
    pub comparison: ComparisonMetrics,
}

/// Comparison metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    pub speedup_factor: f64,
    pub size_reduction_percent: f64,
    pub retrieval_overlap_percent: f64,
    pub chunks_in_common: usize,
    pub traditional_only: usize,
    pub utp_only: usize,
}

/// Retrieved chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: i32,
    pub document_id: i32,
    pub document_title: String,
    pub chunk_index: i32,
    pub content: String,
    pub similarity_score: f32,
    pub rank: usize,  // 1-indexed rank in results
}

/// Aggregate vector statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VectorStatistics {
    pub id: i32,
    pub total_documents: i32,
    pub total_chunks: i32,

    // Storage metrics
    pub total_f32_size_mb: f32,
    pub total_int8_size_mb: f32,
    pub avg_compression_ratio: f32,
    pub storage_savings_percent: f32,

    // Query metrics
    pub total_queries: i32,
    pub avg_traditional_latency_ms: f32,
    pub avg_utp_latency_ms: f32,
    pub avg_speedup_factor: f32,
    pub utp_cache_hit_rate: f32,

    // Accuracy metrics
    pub avg_retrieval_overlap: f32,

    pub last_updated: DateTime<Utc>,
}

/// Document with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWithStats {
    pub document: Document,
    pub chunk_count: i32,
    pub f32_size_mb: f32,
    pub int8_size_mb: f32,
    pub compression_ratio: f32,
    pub storage_savings_mb: f32,
}

/// PDF upload request (multipart form data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadDocumentRequest {
    pub title: String,
    // File is handled separately via multipart
}

/// PDF upload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadDocumentResponse {
    pub document: Document,
    pub chunks_created: i32,
    pub f32_size_mb: f32,
    pub int8_size_mb: f32,
    pub compression_ratio: f32,
    pub processing_time_ms: u128,
}
