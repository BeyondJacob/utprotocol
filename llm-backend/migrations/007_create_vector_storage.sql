-- Vector Storage for RAG: Traditional vs UTP Comparison
-- This schema supports side-by-side comparison of:
-- 1. Traditional approach: Full F32 embeddings
-- 2. UTP approach: Compressed Int8 embeddings with semantic cache

-- Documents table (uploaded PDFs, text files, etc.)
CREATE TABLE IF NOT EXISTS documents (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_type TEXT NOT NULL,  -- 'pdf', 'txt', 'md', etc.
    file_size_bytes INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL DEFAULT 0,
    upload_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB  -- Store arbitrary metadata (author, date, etc.)
);

CREATE INDEX idx_documents_upload_date ON documents(upload_date DESC);

-- Document chunks (split text for embedding)
CREATE TABLE IF NOT EXISTS document_chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,  -- Order within document
    content TEXT NOT NULL,  -- Actual text chunk
    chunk_size INTEGER NOT NULL,  -- Character count

    -- Traditional F32 embedding (baseline)
    embedding_f32 REAL[],  -- PostgreSQL array for F32 vectors
    embedding_f32_dimension INTEGER,
    embedding_f32_size_bytes INTEGER,

    -- UTP compressed embedding (Int8)
    embedding_int8 BYTEA,  -- Binary storage for compressed embedding
    embedding_int8_dimension INTEGER,
    embedding_int8_size_bytes INTEGER,
    compression_ratio REAL,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(document_id, chunk_index)
);

CREATE INDEX idx_document_chunks_document ON document_chunks(document_id);
CREATE INDEX idx_document_chunks_order ON document_chunks(document_id, chunk_index);

-- RAG queries table (track all queries for benchmarking)
CREATE TABLE IF NOT EXISTS rag_queries (
    id SERIAL PRIMARY KEY,
    query_text TEXT NOT NULL,
    query_embedding_f32 REAL[],  -- Store query embedding for analysis

    -- Traditional approach metrics
    traditional_latency_ms INTEGER,
    traditional_top_k_chunks INTEGER[],  -- Array of chunk IDs
    traditional_total_size_bytes INTEGER,

    -- UTP approach metrics
    utp_latency_ms INTEGER,
    utp_top_k_chunks INTEGER[],  -- Array of chunk IDs
    utp_total_size_bytes INTEGER,
    utp_cache_hit BOOLEAN DEFAULT FALSE,
    utp_similarity_score REAL,

    -- Comparison metrics
    speedup_factor REAL,
    size_reduction_percent REAL,
    retrieval_overlap_percent REAL,  -- % of chunks found by both methods

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rag_queries_created ON rag_queries(created_at DESC);
CREATE INDEX idx_rag_queries_cache_hit ON rag_queries(utp_cache_hit);

-- Aggregate statistics table
CREATE TABLE IF NOT EXISTS vector_statistics (
    id SERIAL PRIMARY KEY,
    total_documents INTEGER NOT NULL DEFAULT 0,
    total_chunks INTEGER NOT NULL DEFAULT 0,

    -- Storage metrics
    total_f32_size_mb REAL NOT NULL DEFAULT 0,
    total_int8_size_mb REAL NOT NULL DEFAULT 0,
    avg_compression_ratio REAL NOT NULL DEFAULT 0,
    storage_savings_percent REAL NOT NULL DEFAULT 0,

    -- Query metrics
    total_queries INTEGER NOT NULL DEFAULT 0,
    avg_traditional_latency_ms REAL NOT NULL DEFAULT 0,
    avg_utp_latency_ms REAL NOT NULL DEFAULT 0,
    avg_speedup_factor REAL NOT NULL DEFAULT 0,
    utp_cache_hit_rate REAL NOT NULL DEFAULT 0,

    -- Accuracy metrics
    avg_retrieval_overlap REAL NOT NULL DEFAULT 0,  -- How often same chunks retrieved

    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert initial stats row
INSERT INTO vector_statistics (id) VALUES (1)
ON CONFLICT (id) DO NOTHING;

-- Trigger to update statistics on chunk insert
CREATE OR REPLACE FUNCTION update_vector_statistics()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE vector_statistics
    SET
        total_chunks = (SELECT COUNT(*) FROM document_chunks),
        total_f32_size_mb = (SELECT COALESCE(SUM(embedding_f32_size_bytes), 0) / 1024.0 / 1024.0 FROM document_chunks),
        total_int8_size_mb = (SELECT COALESCE(SUM(embedding_int8_size_bytes), 0) / 1024.0 / 1024.0 FROM document_chunks),
        avg_compression_ratio = (SELECT COALESCE(AVG(compression_ratio), 0) FROM document_chunks WHERE compression_ratio IS NOT NULL),
        storage_savings_percent = (
            SELECT
                CASE
                    WHEN SUM(embedding_f32_size_bytes) > 0
                    THEN ((SUM(embedding_f32_size_bytes) - SUM(embedding_int8_size_bytes)) / SUM(embedding_f32_size_bytes)::REAL) * 100
                    ELSE 0
                END
            FROM document_chunks
        ),
        last_updated = NOW()
    WHERE id = 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_vector_statistics
AFTER INSERT OR UPDATE OR DELETE ON document_chunks
FOR EACH STATEMENT
EXECUTE FUNCTION update_vector_statistics();
