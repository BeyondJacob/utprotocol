/// Repository for vector storage operations (RAG comparison)

use super::vector_models::*;
use anyhow::Result;
use sqlx::PgPool;

pub struct VectorRepository {
    pool: PgPool,
}

impl VectorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new document
    pub async fn create_document(
        &self,
        title: &str,
        file_name: &str,
        file_type: &str,
        file_size_bytes: i32,
        metadata: Option<serde_json::Value>,
    ) -> Result<Document> {
        let document = sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (title, file_name, file_type, file_size_bytes, metadata)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(title)
        .bind(file_name)
        .bind(file_type)
        .bind(file_size_bytes)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(document)
    }

    /// Get document by ID
    pub async fn get_document(&self, id: i32) -> Result<Option<Document>> {
        let document = sqlx::query_as::<_, Document>(
            r#"
            SELECT * FROM documents WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(document)
    }

    /// List all documents
    pub async fn list_documents(&self) -> Result<Vec<Document>> {
        let documents = sqlx::query_as::<_, Document>(
            r#"
            SELECT * FROM documents ORDER BY upload_date DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(documents)
    }

    /// Delete document and all its chunks
    pub async fn delete_document(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a document chunk with dual embeddings
    pub async fn create_chunk(&self, req: CreateChunkRequest) -> Result<DocumentChunk> {
        let embedding_f32_dimension = req.embedding_f32.len() as i32;
        let embedding_f32_size_bytes = (req.embedding_f32.len() * 4) as i32; // f32 = 4 bytes

        let embedding_int8_dimension = embedding_f32_dimension; // Same dimension
        let embedding_int8_size_bytes = req.embedding_int8.len() as i32;

        let compression_ratio =
            embedding_f32_size_bytes as f32 / embedding_int8_size_bytes as f32;

        let chunk = sqlx::query_as::<_, DocumentChunk>(
            r#"
            INSERT INTO document_chunks (
                document_id, chunk_index, content, chunk_size,
                embedding_f32, embedding_f32_dimension, embedding_f32_size_bytes,
                embedding_int8, embedding_int8_dimension, embedding_int8_size_bytes,
                compression_ratio
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(req.document_id)
        .bind(req.chunk_index)
        .bind(&req.content)
        .bind(req.content.len() as i32)
        .bind(&req.embedding_f32)
        .bind(embedding_f32_dimension)
        .bind(embedding_f32_size_bytes)
        .bind(&req.embedding_int8)
        .bind(embedding_int8_dimension)
        .bind(embedding_int8_size_bytes)
        .bind(compression_ratio)
        .fetch_one(&self.pool)
        .await?;

        Ok(chunk)
    }

    /// Get all chunks for a document
    #[allow(dead_code)]
    pub async fn get_document_chunks(&self, document_id: i32) -> Result<Vec<DocumentChunk>> {
        let chunks = sqlx::query_as::<_, DocumentChunk>(
            r#"
            SELECT * FROM document_chunks
            WHERE document_id = $1
            ORDER BY chunk_index
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(chunks)
    }

    /// Update document chunk count
    pub async fn update_document_chunk_count(&self, document_id: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE documents
            SET total_chunks = (
                SELECT COUNT(*) FROM document_chunks WHERE document_id = $1
            )
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Search chunks using traditional F32 embeddings (cosine similarity via SQL)
    /// Note: This is a simple implementation. For production, use pgvector extension.
    #[allow(dead_code)]
    pub async fn search_traditional(
        &self,
        query_embedding: &[f32],
        top_k: i32,
    ) -> Result<Vec<(DocumentChunk, f32)>> {
        // For now, we'll fetch all chunks and compute similarity in Rust
        // In production, use pgvector's <-> operator for efficient similarity search
        let all_chunks = sqlx::query_as::<_, DocumentChunk>(
            r#"
            SELECT * FROM document_chunks
            WHERE embedding_f32 IS NOT NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Compute cosine similarity for each chunk
        let mut results: Vec<(DocumentChunk, f32)> = all_chunks
            .into_iter()
            .filter_map(|chunk| {
                if let Some(embedding) = &chunk.embedding_f32 {
                    let similarity = cosine_similarity(query_embedding, embedding);
                    Some((chunk, similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        results.truncate(top_k as usize);

        Ok(results)
    }

    /// Record a RAG query for benchmarking
    #[allow(dead_code)]
    pub async fn record_rag_query(
        &self,
        query_text: &str,
        query_embedding: &[f32],
        traditional_latency_ms: Option<i32>,
        traditional_chunks: Option<Vec<i32>>,
        traditional_size: Option<i32>,
        utp_latency_ms: Option<i32>,
        utp_chunks: Option<Vec<i32>>,
        utp_size: Option<i32>,
        utp_cache_hit: Option<bool>,
        utp_similarity: Option<f32>,
    ) -> Result<RagQuery> {
        // Calculate comparison metrics
        let speedup_factor = if let (Some(trad), Some(utp)) =
            (traditional_latency_ms, utp_latency_ms)
        {
            if utp > 0 {
                Some(trad as f32 / utp as f32)
            } else {
                None
            }
        } else {
            None
        };

        let size_reduction = if let (Some(trad), Some(utp)) = (traditional_size, utp_size) {
            if trad > 0 {
                Some(((trad - utp) as f32 / trad as f32) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        // Calculate retrieval overlap
        let overlap = if let (Some(ref trad_chunks), Some(ref utp_chunks)) =
            (&traditional_chunks, &utp_chunks)
        {
            let common: Vec<&i32> = trad_chunks
                .iter()
                .filter(|id| utp_chunks.contains(id))
                .collect();
            let total = trad_chunks.len().max(utp_chunks.len());
            if total > 0 {
                Some((common.len() as f32 / total as f32) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        let query = sqlx::query_as::<_, RagQuery>(
            r#"
            INSERT INTO rag_queries (
                query_text, query_embedding_f32,
                traditional_latency_ms, traditional_top_k_chunks, traditional_total_size_bytes,
                utp_latency_ms, utp_top_k_chunks, utp_total_size_bytes,
                utp_cache_hit, utp_similarity_score,
                speedup_factor, size_reduction_percent, retrieval_overlap_percent
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(query_text)
        .bind(query_embedding)
        .bind(traditional_latency_ms)
        .bind(&traditional_chunks)
        .bind(traditional_size)
        .bind(utp_latency_ms)
        .bind(&utp_chunks)
        .bind(utp_size)
        .bind(utp_cache_hit)
        .bind(utp_similarity)
        .bind(speedup_factor)
        .bind(size_reduction)
        .bind(overlap)
        .fetch_one(&self.pool)
        .await?;

        Ok(query)
    }

    /// Get vector statistics
    pub async fn get_statistics(&self) -> Result<VectorStatistics> {
        let stats = sqlx::query_as::<_, VectorStatistics>(
            r#"
            SELECT * FROM vector_statistics WHERE id = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// Get document with statistics
    pub async fn get_document_with_stats(&self, id: i32) -> Result<Option<DocumentWithStats>> {
        let document = self.get_document(id).await?;

        if let Some(doc) = document {
            // Get chunk statistics using regular query (not macro) to avoid compile-time DB requirement
            let stats: (i64, f64, f64, f64) = sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) as chunk_count,
                    COALESCE(SUM(embedding_f32_size_bytes), 0) / 1024.0 / 1024.0 as f32_size_mb,
                    COALESCE(SUM(embedding_int8_size_bytes), 0) / 1024.0 / 1024.0 as int8_size_mb,
                    COALESCE(AVG(compression_ratio), 0) as avg_compression_ratio
                FROM document_chunks
                WHERE document_id = $1
                "#,
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            let f32_size = stats.1 as f32;
            let int8_size = stats.2 as f32;
            let compression_ratio = stats.3 as f32;
            let savings = f32_size - int8_size;

            Ok(Some(DocumentWithStats {
                document: doc,
                chunk_count: stats.0 as i32,
                f32_size_mb: f32_size,
                int8_size_mb: int8_size,
                compression_ratio,
                storage_savings_mb: savings,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Compute cosine similarity between two embeddings
#[allow(dead_code)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}
