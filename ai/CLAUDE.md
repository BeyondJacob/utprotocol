# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Universal Thought Protocol (UTP) is a production-grade full-stack LLM chat application demonstrating a novel Layer-3 compression and semantic caching protocol for LLM embeddings. The system features a Rust backend (Axum) with PostgreSQL persistence and a Next.js 15 frontend with shadcn/ui components.

**Key Innovation**: UTP reduces embedding storage by 4-8x through quantization (F32/F16/Int8/Int4) and achieves 50x+ speedup on cache hits via semantic similarity matching.

## Architecture

The codebase is split into two main directories:

- **llm-backend/** - Rust backend (Axum web server on port 3001)
- **llm-frontend/** - Next.js 15 frontend (React 19 on port 3000)

### Backend Architecture (llm-backend/src/)

```
main.rs                      # HTTP server, API routes, initialization
config.rs                    # TOML configuration loading
├── db/                      # Database layer (SQLx + PostgreSQL)
│   ├── models.rs            # Conversation, Message models
│   ├── repository.rs        # CRUD operations
│   ├── vector_models.rs     # Document, Chunk models for RAG
│   └── vector_repository.rs # Vector storage operations
├── providers/               # LLM provider implementations
│   ├── mod.rs               # ModelProvider trait & registry
│   ├── ollama.rs            # Ollama local provider
│   ├── groq.rs              # Groq cloud provider
│   ├── rate_limiter.rs      # Token bucket rate limiting (governor)
│   └── circuit_breaker.rs   # Failfast pattern
├── utp/                     # Universal Thought Protocol core
│   ├── protocol.rs          # Core types (UtpHeader, CompressedEmbedding, Precision)
│   ├── compression.rs       # Layer3Compressor (quantization algorithms)
│   ├── cache.rs             # EmbeddingCache (LRU + TTL, DashMap-based)
│   ├── embeddings.rs        # Real embedding generation via Ollama
│   ├── similarity.rs        # Cosine similarity for semantic matching
│   ├── middleware.rs        # UtpMiddleware (wraps providers)
│   └── metrics.rs           # Performance tracking
├── agents/                  # Agent flow system (for multi-step tasks)
│   ├── models.rs            # Agent, Task, Flow models
│   ├── registry.rs          # Agent registration
│   ├── execution.rs         # Task execution logic
│   └── flow_executor.rs     # Flow orchestration
├── rag_handlers.rs          # RAG API endpoints (upload, query, stats)
└── pdf_processor.rs         # PDF text extraction

config.toml                  # Configuration file (cache, rate limits, etc.)
```

**Key UTP Flow**:
1. Request arrives → UtpMiddleware checks semantic hash in cache
2. Cache miss → Generate embedding via Ollama → Compress (Int8) → Store → Return
3. Cache hit → Decompress → Return (<1ms vs ~75ms)

### Frontend Architecture (llm-frontend/)

```
app/
├── page.tsx                      # Main page (renders MultiChatInterface)
├── layout.tsx                    # Root layout
└── globals.css                   # Tailwind styles + CSS variables

components/
├── multi-chat-interface.tsx      # 3-window chat layout manager
├── chat-window.tsx               # Individual chat window (model selector + messages)
├── chat-sidebar.tsx              # Conversation history sidebar
├── message-list.tsx              # Message display with UTP metric badges
├── chat-input.tsx                # Text input + send button
├── model-selector.tsx            # Dropdown with download/delete actions
├── pdf-upload.tsx                # RAG document upload UI
├── rag-comparison.tsx            # Side-by-side Traditional vs UTP query comparison
├── rag-statistics.tsx            # Real-time UTP performance dashboard
├── agent-flow/                   # Agent visualization components (xyflow/react)
└── ui/                           # shadcn/ui primitives (button, card, select, etc.)
```

**Component Data Flow**:
- `MultiChatInterface` manages windows and loads conversations
- `ChatWindow` sends messages to `/chat` endpoint with `use_utp` flag
- `MessageList` displays `utp_metadata` as badges (cache hit, compression ratio, precision)
- `RagComparison` queries `/rag/query` twice (Traditional vs UTP) and displays side-by-side results

## Development Commands

### Backend (llm-backend/)

```bash
# Development
cargo run                                 # Start server on http://127.0.0.1:3001
RUST_LOG=debug cargo run                  # Run with debug logging

# Testing
cargo test                                # Run all tests
cargo test -- --nocapture                 # Run with output

# Building
cargo build --release                     # Optimized build
./target/release/llm-backend              # Run release binary

# Database
psql utprotocol                           # Connect to database
psql utprotocol < migrations/XXX.sql      # Run specific migration
```

**Environment Variables**: Create `.env` in project root:
```
DATABASE_URL=postgres://username@localhost/utprotocol
GROQ_API_KEY=your_api_key_here  # Optional
RUST_LOG=info                    # Log level: error|warn|info|debug|trace
```

### Frontend (llm-frontend/)

```bash
# Development
npm install                               # Install dependencies (first time)
npm run dev                               # Start dev server on http://localhost:3000

# Building
npm run build                             # Production build
npm start                                 # Start production server

# Linting
npm run lint                              # Run ESLint
npm run lint -- --fix                     # Auto-fix issues
```

### Full Stack Setup

```bash
# 1. Prerequisites
brew install ollama postgresql@17
brew services start ollama postgresql@17

# 2. Database
createdb utprotocol
cd llm-backend && psql utprotocol < migrations/001_*.sql  # Run all migrations

# 3. Ollama models
ollama pull nomic-embed-text              # Required for embeddings
ollama pull llama3.2:1b                   # Small test model
ollama pull gpt-oss:20b                   # Optional (13GB)

# 4. Start backend
cd llm-backend && cargo run

# 5. Start frontend (new terminal)
cd llm-frontend && npm run dev

# 6. Open browser
open http://localhost:3000
```

## Key Technical Concepts

### UTP Protocol Precision Levels

| Precision | Size (768-dim) | Compression | Use Case |
|-----------|----------------|-------------|----------|
| F32 | 3072 bytes | 1x (baseline) | Perfect quality, no compression |
| F16 | 1536 bytes | 2x | High quality, half-precision floats |
| Int8 | ~800 bytes | 4x | Production default (dynamic scaling) |
| Int4 | ~400 bytes | 8x | Maximum compression (nibble packing) |

**Default**: Int8 for cache storage (best quality/compression tradeoff)

### Database Schema

**conversations** table:
- `id`, `title`, `model`, `utp_enabled`, `created_at`, `updated_at`

**messages** table:
- `id`, `conversation_id`, `role`, `content`, `model`, `latency_ms`
- `prompt_tokens`, `completion_tokens`, `tokens_per_second`, `total_tokens`
- `utp_metadata` (JSONB) - Contains: `used_utp`, `cache_hit`, `compression_ratio`, `latency_us`, `original_size`, `compressed_size`, `precision_used`

**documents** table (RAG):
- `id`, `title`, `content_type`, `source`, `metadata`, `created_at`

**chunks** table (RAG):
- `id`, `document_id`, `content`, `chunk_index`, `token_count`
- `embedding` (vector) - Uncompressed embeddings for traditional path
- `compressed_embedding` (bytea) - UTP compressed embeddings

### API Endpoints

**Chat**: `POST /chat`
```json
{
  "model": "gpt-oss:20b",
  "message": "Your message",
  "conversation_id": 1,
  "use_utp": true
}
```

**Models**: `GET /models` - Returns all available models with download status

**Download Model**: `POST /download-model` - Pulls model via Ollama

**Delete Model**: `POST /delete-model` - Removes model from disk

**UTP Stats**: `GET /utp/stats` - Returns cache and performance metrics

**RAG Query**: `POST /rag/query`
```json
{
  "query": "What is UTP?",
  "top_k": 3,
  "use_utp": true
}
```

**RAG Upload**: `POST /documents/upload` - Multipart form (title + PDF file)

**RAG Stats**: `GET /rag/statistics` - Returns storage and performance comparison

### Configuration (llm-backend/config.toml)

All production tunables are in `config.toml`:
- **Cache**: `max_entries`, `ttl_seconds`, `eviction_policy`, `similarity_threshold`
- **Providers**: Timeout, retries, rate limits, circuit breaker thresholds
- **Server**: Host, port

Changes require backend restart (`cargo run`).

### Testing UTP

```bash
# Traditional (no UTP)
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2:1b","message":"Hello","use_utp":false}'

# UTP first time (cache miss)
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2:1b","message":"Hello","use_utp":true}'

# UTP second time (cache hit - same message)
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2:1b","message":"Hello","use_utp":true}'

# Check stats
curl http://localhost:3001/utp/stats | jq
```

## Important Implementation Details

### UTP Compression Algorithm (src/utp/compression.rs)

**Int8 Quantization**:
1. Find max absolute value in embedding vector
2. Calculate scale: `max_abs / 127.0`
3. Quantize: `(value / scale).round().clamp(-128, 127) as i8`
4. Store scale factor for decompression

**Int4 Quantization**:
1. Scale to 4-bit range (-8 to 7)
2. Pack two 4-bit values per byte (nibbles): `(high << 4) | low`
3. Reduces storage by 8x but loses more precision

### Semantic Cache Matching (src/utp/cache.rs)

1. Generate embedding for incoming query
2. Compare to all cached embeddings using cosine similarity
3. If similarity >= `similarity_threshold` (default 0.92): Cache hit
4. Otherwise: Cache miss, process query and cache result
5. LRU eviction when cache reaches `max_entries`

**Thread-safe**: Uses `DashMap` for concurrent access without locks.

### Multi-Provider Architecture (src/providers/)

All providers implement `ModelProvider` trait:
```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn generate(&self, model: &str, prompt: &str) -> Result<GenerateResponse>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    // ...
}
```

**Ollama**: Local models, no rate limits, longer timeout (120s)
**Groq**: Cloud API, rate limited (60 req/min), circuit breaker enabled

### Frontend Type Safety

All backend responses are typed in TypeScript:
```typescript
type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string;
  latency?: number;
  utp_metadata?: UtpMetadata;  // Contains compression stats
};

type UtpMetadata = {
  used_utp: boolean;
  cache_hit: boolean;
  compression_ratio: number;
  latency_us: number;
  original_size: number;
  compressed_size: number;
  precision_used: string;
};
```

## Common Development Patterns

### Adding a New API Endpoint (Backend)

1. Add handler function in `main.rs`:
```rust
async fn my_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MyRequest>,
) -> Result<Json<MyResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Implementation
    Ok(Json(MyResponse { ... }))
}
```

2. Register route:
```rust
let app = Router::new()
    .route("/my-endpoint", post(my_handler))
    .with_state(state);
```

### Adding a New Frontend Component

1. Create component file in `components/`:
```tsx
export function MyComponent({ prop1, prop2 }: MyComponentProps) {
  const [state, setState] = useState<Type>();

  return (
    <div className="...">
      {/* JSX */}
    </div>
  );
}
```

2. Import and use in parent component:
```tsx
import { MyComponent } from "@/components/my-component";

<MyComponent prop1={value} prop2={value} />
```

### Adding a Database Migration

1. Create migration file: `llm-backend/migrations/00X_description.sql`
2. Write SQL (CREATE TABLE, ALTER TABLE, etc.)
3. Run: `psql utprotocol < migrations/00X_description.sql`
4. Update `db/models.rs` and `db/repository.rs` for new schema

### Modifying UTP Compression

Edit `llm-backend/src/utp/compression.rs`:
- `compress()` - Add new precision variant
- `decompress()` - Add corresponding decompression logic
- Update `Precision` enum in `protocol.rs`
- Update frontend `UtpMetadata` type if needed

## Troubleshooting

**Backend won't start**:
- Check PostgreSQL: `pg_isready`
- Check Ollama: `curl http://localhost:11434/api/tags`
- Check DATABASE_URL in `.env`

**Frontend build errors**:
```bash
rm -rf node_modules .next
npm install
npm run dev
```

**Port conflicts**:
```bash
lsof -ti:3001 | xargs kill -9  # Backend
lsof -ti:3000 | xargs kill -9  # Frontend
```

**Database connection issues**:
```bash
brew services list | grep postgresql
psql utprotocol -c "SELECT 1"  # Test connection
```

**UTP cache not working**:
- Check logs: Should see "✅ UTP middleware initialized"
- Verify embedding generation: Check logs for "Generated embedding"
- Test with curl commands above to verify cache hit on second request

## Performance Benchmarks

**Compression** (768-dimensional embeddings):
- F32: 3072 bytes (baseline)
- F16: 1536 bytes (2x, <1% quality loss)
- Int8: 772 bytes (3.98x, ~2% quality loss)
- Int4: 388 bytes (7.92x, ~5% quality loss)

**Cache Performance**:
- Traditional embedding API: ~75ms
- UTP cache hit: <1ms (50x+ speedup)
- Typical hit rate: 25-70% (depends on query patterns)

**Overall Improvement** (with 50% cache hit rate):
- Latency: 2x faster
- Storage: 1.59x smaller
- Cost savings: ~50% for cloud providers
