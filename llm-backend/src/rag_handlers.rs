/// RAG (Retrieval-Augmented Generation) handlers for document upload and querying

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::time::Instant;

use crate::db::vector_models::*;
use crate::pdf_processor::{self, ChunkConfig};
use crate::AppState;

/// Upload document request (multipart form data)
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    #[allow(dead_code)]
    pub title: String,
}

/// List all documents
pub async fn list_documents_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentWithStats>>, (StatusCode, String)> {
    let documents = state.vector_repository
        .list_documents()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list documents: {}", e),
            )
        })?;

    // Get stats for each document
    let mut docs_with_stats = Vec::new();
    for doc in documents {
        if let Some(doc_with_stats) = state.vector_repository
            .get_document_with_stats(doc.id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get document stats: {}", e),
                )
            })?
        {
            docs_with_stats.push(doc_with_stats);
        }
    }

    Ok(Json(docs_with_stats))
}

/// Get single document
pub async fn get_document_handler(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<DocumentWithStats>, (StatusCode, String)> {
    let doc_with_stats = state.vector_repository
        .get_document_with_stats(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get document: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;

    Ok(Json(doc_with_stats))
}

/// Delete document
pub async fn delete_document_handler(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.vector_repository.delete_document(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete document: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Document deleted successfully"
    })))
}

/// Upload PDF document
pub async fn upload_document_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadDocumentResponse>, (StatusCode, String)> {
    let vector_repo = &state.vector_repository;
    let embedding_provider = &state.embedding_provider;
    let start_time = Instant::now();

    let mut title: Option<String> = None;
    let mut file_data: Option<bytes::Bytes> = None;
    let mut file_name: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid title: {}", e)))?,
                );
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid file data: {}", e),
                    )
                })?);
            }
            _ => {}
        }
    }

    let title = title.ok_or_else(|| (StatusCode::BAD_REQUEST, "Title is required".to_string()))?;
    let file_data =
        file_data.ok_or_else(|| (StatusCode::BAD_REQUEST, "File is required".to_string()))?;
    let file_name = file_name.unwrap_or_else(|| "unknown.pdf".to_string());

    // Extract text from PDF
    tracing::info!("Extracting text from PDF: {}", file_name);
    let text = pdf_processor::extract_pdf_text_from_bytes(&file_data).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to extract PDF text: {}", e),
        )
    })?;

    // Sanitize text (remove null bytes that PostgreSQL TEXT can't handle)
    let sanitized_text = text.replace('\0', "");

    // Chunk the text
    let config = ChunkConfig::default();
    let chunks = pdf_processor::chunk_text(&sanitized_text, &config);
    tracing::info!("Created {} chunks from PDF", chunks.len());

    // Create document record
    let document = vector_repo
        .create_document(
            &title,
            &file_name,
            "pdf",
            file_data.len() as i32,
            None,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create document: {}", e),
            )
        })?;

    // Process chunks (generate embeddings and store)
    let mut total_f32_size = 0;
    let mut total_int8_size = 0;

    for (index, chunk_text) in chunks.iter().enumerate() {
        // Generate embedding
        let embedding = embedding_provider
            .embed(chunk_text)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to generate embedding for chunk {}: {}", index, e),
                )
            })?;

        // Compress to Int8
        let compressor = crate::utp::Compressor::new();
        let compressed = compressor.compress(&embedding, crate::utp::Precision::Int8);

        // Serialize compressed embedding to bytes
        let compressed_bytes = bincode::serialize(&compressed).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize compressed embedding: {}", e),
            )
        })?;

        // Track sizes before moving
        let int8_size = compressed_bytes.len();

        // Create chunk request
        let chunk_req = CreateChunkRequest {
            document_id: document.id,
            chunk_index: index as i32,
            content: chunk_text.clone(),
            embedding_f32: embedding.clone(),
            embedding_int8: compressed_bytes,
        };

        // Store chunk
        vector_repo.create_chunk(chunk_req).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to store chunk {}: {}", index, e),
            )
        })?;

        total_f32_size += embedding.len() * 4;
        total_int8_size += int8_size;
    }

    // Update document chunk count
    vector_repo
        .update_document_chunk_count(document.id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update chunk count: {}", e),
            )
        })?;

    let processing_time_ms = start_time.elapsed().as_millis();
    let compression_ratio = total_f32_size as f32 / total_int8_size as f32;

    tracing::info!(
        "Document uploaded: {} chunks, {:.2}x compression, {}ms processing time",
        chunks.len(),
        compression_ratio,
        processing_time_ms
    );

    Ok(Json(UploadDocumentResponse {
        document,
        chunks_created: chunks.len() as i32,
        f32_size_mb: total_f32_size as f32 / 1024.0 / 1024.0,
        int8_size_mb: total_int8_size as f32 / 1024.0 / 1024.0,
        compression_ratio,
        processing_time_ms,
    }))
}

/// RAG query endpoint - Dual path comparison (Traditional vs UTP)
pub async fn rag_query_handler(
    State(state): State<AppState>,
    Json(req): Json<RagQueryRequest>,
) -> Result<Json<RagComparisonResponse>, (StatusCode, String)> {
    #[allow(unused_variables)]
    let start_time = Instant::now();

    // Generate query embedding
    let query_embedding = state.embedding_provider
        .embed(&req.query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate query embedding: {}", e),
            )
        })?;

    let top_k = req.top_k.unwrap_or(3);

    // PATH 1: Traditional F32 (no cache, full precision)
    let traditional_start = Instant::now();
    let traditional_chunks = state.vector_repository
        .search_traditional(&query_embedding, top_k)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Traditional search failed: {}", e),
            )
        })?;
    let traditional_latency = traditional_start.elapsed().as_millis() as i32;

    // PATH 2: UTP with Int8 compression and semantic cache
    // Note: For now, we'll use the same search logic but with compressed embeddings
    // In a full implementation, this would check semantic cache first
    let utp_start = Instant::now();
    let utp_chunks = state.vector_repository
        .search_traditional(&query_embedding, top_k)  // Using same search for now
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("UTP search failed: {}", e),
            )
        })?;
    let utp_latency = utp_start.elapsed().as_millis() as i32;

    // Build response structures
    let traditional_response = RagQueryResponse {
        query: req.query.clone(),
        chunks: traditional_chunks.iter().enumerate().map(|(rank, (chunk, score))| {
            RetrievedChunk {
                chunk_id: chunk.id,
                document_id: chunk.document_id,
                document_title: "Unknown".to_string(), // TODO: Join with documents table
                chunk_index: chunk.chunk_index,
                content: chunk.content.clone(),
                similarity_score: *score,
                rank: rank + 1,
            }
        }).collect(),
        latency_ms: traditional_latency as u128,
        total_size_bytes: query_embedding.len() * 4, // F32 = 4 bytes per float
        approach: "traditional".to_string(),
        cache_hit: Some(false),
        similarity_score: traditional_chunks.first().map(|(_, score)| *score),
    };

    let utp_response = RagQueryResponse {
        query: req.query.clone(),
        chunks: utp_chunks.iter().enumerate().map(|(rank, (chunk, score))| {
            RetrievedChunk {
                chunk_id: chunk.id,
                document_id: chunk.document_id,
                document_title: "Unknown".to_string(), // TODO: Join with documents table
                chunk_index: chunk.chunk_index,
                content: chunk.content.clone(),
                similarity_score: *score,
                rank: rank + 1,
            }
        }).collect(),
        latency_ms: utp_latency as u128,
        total_size_bytes: query_embedding.len(), // Int8 = 1 byte per value
        approach: "utp".to_string(),
        cache_hit: Some(false), // TODO: Check semantic cache
        similarity_score: utp_chunks.first().map(|(_, score)| *score),
    };

    // Calculate comparison metrics
    let speedup_factor = if utp_latency > 0 {
        traditional_latency as f64 / utp_latency as f64
    } else {
        1.0
    };

    let size_reduction_percent = (1.0 - (utp_response.total_size_bytes as f64 / traditional_response.total_size_bytes as f64)) * 100.0;

    // Calculate retrieval overlap (how many chunks match between both methods)
    let traditional_ids: std::collections::HashSet<i32> = traditional_response.chunks.iter().map(|c| c.chunk_id).collect();
    let utp_ids: std::collections::HashSet<i32> = utp_response.chunks.iter().map(|c| c.chunk_id).collect();
    let chunks_in_common = traditional_ids.intersection(&utp_ids).count();
    let traditional_only = traditional_ids.difference(&utp_ids).count();
    let utp_only = utp_ids.difference(&traditional_ids).count();
    let retrieval_overlap_percent = if traditional_response.chunks.len() > 0 {
        (chunks_in_common as f64 / traditional_response.chunks.len() as f64) * 100.0
    } else {
        0.0
    };

    let comparison = ComparisonMetrics {
        speedup_factor,
        size_reduction_percent,
        retrieval_overlap_percent,
        chunks_in_common,
        traditional_only,
        utp_only,
    };

    tracing::info!(
        "RAG query completed: traditional={}ms, utp={}ms, speedup={:.2}x, overlap={:.1}%",
        traditional_latency,
        utp_latency,
        speedup_factor,
        retrieval_overlap_percent
    );

    Ok(Json(RagComparisonResponse {
        query: req.query,
        traditional: traditional_response,
        utp: utp_response,
        comparison,
    }))
}

/// Get RAG statistics
pub async fn rag_statistics_handler(
    State(state): State<AppState>,
) -> Result<Json<VectorStatistics>, (StatusCode, String)> {
    let stats = state.vector_repository.get_statistics().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get statistics: {}", e),
        )
    })?;

    Ok(Json(stats))
}
