/// RAG (Retrieval-Augmented Generation) handlers for document upload and querying

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::db::vector_models::*;
use crate::db::vector_repository::{VectorRepository, CreateChunkRequest};
use crate::pdf_processor::{self, ChunkConfig};
use crate::utp::EmbeddingProvider;
use crate::AppState;

/// Upload document request (multipart form data)
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
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
    State(vector_repo): State<Arc<VectorRepository>>,
) -> Result<Json<DocumentWithStats>, (StatusCode, String)> {
    let doc_with_stats = vector_repo
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
    State(vector_repo): State<Arc<VectorRepository>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    vector_repo.delete_document(id).await.map_err(|e| {
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
/// This will be a simplified version for now - we'll need to handle multipart later
pub async fn upload_document_handler(
    State(vector_repo): State<Arc<VectorRepository>>,
    State(embedding_provider): State<Arc<dyn EmbeddingProvider>>,
    mut multipart: Multipart,
) -> Result<Json<UploadDocumentResponse>, (StatusCode, String)> {
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

    // Chunk the text
    let config = ChunkConfig::default();
    let chunks = pdf_processor::chunk_text(&text, &config);
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
        total_int8_size += compressed_bytes.len();
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

/// RAG query endpoint (placeholder - will implement dual path next)
pub async fn rag_query_handler(
    State(_vector_repo): State<Arc<VectorRepository>>,
    Json(_req): Json<RagQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // TODO: Implement dual path query (Traditional + UTP)
    Ok(Json(serde_json::json!({
        "message": "RAG query endpoint - implementation coming next",
        "status": "pending"
    })))
}

/// Get RAG statistics
pub async fn rag_statistics_handler(
    State(vector_repo): State<Arc<VectorRepository>>,
) -> Result<Json<VectorStatistics>, (StatusCode, String)> {
    let stats = vector_repo.get_statistics().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get statistics: {}", e),
        )
    })?;

    Ok(Json(stats))
}
