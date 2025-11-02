# Docker Setup Summary

This document summarizes the Docker infrastructure created for the Universal Thought Protocol project.

## Files Created

### Configuration Files

1. **`.env.example`** - Environment variable template
   - Database connection (PostgreSQL)
   - Ollama URL configuration (host.docker.internal:11434)
   - Logging levels
   - Server configuration

2. **`.env.groq.example`** - Groq API key template (optional)
   - Cloud provider API key for Groq models

### Docker Ignore Files

3. **`llm-backend/.dockerignore`** - Backend build exclusions
   - Build artifacts (target/, debug/)
   - IDE files (.vscode/, .idea/)
   - Environment files
   - Documentation
   - Git files

4. **`llm-frontend/.dockerignore`** - Frontend build exclusions
   - Dependencies (node_modules/)
   - Build output (.next/, out/)
   - Environment files
   - IDE files
   - Documentation

### Dockerfiles (Multi-stage Builds)

5. **`llm-backend/Dockerfile`** - Rust backend (2 stages)
   - **Stage 1 (Builder)**: rust:1.91-slim (~1.5 GB)
     - Installs build dependencies
     - Caches Cargo dependencies
     - Compiles release binary
   - **Stage 2 (Runtime)**: debian:bookworm-slim (~200 MB final)
     - Runtime dependencies only
     - Non-root user (utprotocol)
     - Includes entrypoint script
     - Health check enabled

6. **`llm-frontend/Dockerfile`** - Next.js frontend (3 stages)
   - **Stage 1 (Dependencies)**: node:20-alpine
     - Installs npm packages
   - **Stage 2 (Builder)**: node:20-alpine
     - Builds Next.js with standalone output
   - **Stage 3 (Runtime)**: node:20-alpine (~150 MB final)
     - Minimal production runtime
     - Non-root user (nextjs)
     - Health check enabled

### Scripts

7. **`llm-backend/entrypoint.sh`** - Backend initialization
   - Sources Groq API key (if exists)
   - Waits for PostgreSQL to be ready (30 attempts, 2s intervals)
   - Runs database migrations automatically
   - Checks Ollama connectivity (non-blocking)
   - Starts backend server

### Docker Compose Files

8. **`docker-compose.dev.yaml`** - Development environment
   - **Services**:
     - PostgreSQL 18 (port 5432)
     - Backend (port 3001) with debug logging
     - Frontend (port 3000)
   - **Features**:
     - Named volume for PostgreSQL data
     - Health checks with dependencies
     - Extra host mapping for Ollama access
     - Optional source code mounting (commented)

9. **`docker-compose.prod.yaml`** - Production environment
   - **Services**: Same as dev but with production settings
   - **Features**:
     - Environment variable overrides
     - Resource limits (CPU/memory)
     - Log rotation (10MB, 3 files)
     - Always restart policy
     - Backup volume for PostgreSQL
     - Production logging (info level)

### Documentation

10. **`DOCKER.md`** - Comprehensive Docker guide
    - Prerequisites and quick start
    - Architecture diagram
    - Image details and sizes
    - Environment variables reference
    - Common tasks (logs, backup, rebuild)
    - Troubleshooting guide
    - Performance tuning
    - Security considerations
    - CI/CD integration example

11. **`DOCKER_SUMMARY.md`** - This file

### Modified Files

12. **`llm-backend/config.toml`**
    - Changed `host = "127.0.0.1"` → `host = "0.0.0.0"`
    - Added comment explaining Docker vs local development

13. **`llm-frontend/next.config.ts`**
    - Added `output: 'standalone'` for Docker optimization
    - Enables minimal production build

## Key Design Decisions

### Multi-stage Builds

**Why?** Dramatically reduces image sizes:
- Backend: 1.5 GB → 200 MB (87% reduction)
- Frontend: 800 MB → 150 MB (81% reduction)

**How?**
1. Build stage with full toolchain
2. Runtime stage with only compiled artifacts and runtime dependencies

### Entrypoint Script

**Why?** Automates database initialization:
- No manual migration steps required
- Ensures migrations run before server starts
- Handles PostgreSQL readiness gracefully

**How?**
- Waits for PostgreSQL with retries
- Runs all SQL files in migrations/
- Checks Ollama connectivity (warning only)

### Host Machine Ollama Access

**Why?** Avoids running Ollama in Docker (large, GPU-dependent)

**How?**
- Uses `host.docker.internal` (Docker Desktop)
- Maps host gateway in docker-compose
- Configurable via `OLLAMA_URL` environment variable

### Separate Dev/Prod Compose Files

**Why?** Different requirements for development vs production:
- Dev: Debug logging, no resource limits, interactive
- Prod: Info logging, resource limits, always restart

**How?**
- `docker-compose.dev.yaml` for local development
- `docker-compose.prod.yaml` for production deployment

### Security Best Practices

1. **Non-root users**: Both containers run as non-privileged users
2. **Secret management**: Groq API key in separate file (copied as early layer)
3. **Network isolation**: Services communicate via internal Docker network
4. **Resource limits**: Production compose includes CPU/memory constraints
5. **Health checks**: All services have health checks for dependency management

## Quick Start Commands

### Development

```bash
# Start everything
docker compose -f docker-compose.dev.yaml up --build

# Stop everything
docker compose -f docker-compose.dev.yaml down

# View logs
docker compose -f docker-compose.dev.yaml logs -f backend
```

### Production

```bash
# Setup
cp .env.example .env
cp .env.groq.example llm-backend/.env.groq  # Optional

# Start everything (detached)
docker compose -f docker-compose.prod.yaml up --build -d

# Check status
docker compose -f docker-compose.prod.yaml ps

# View logs
docker compose -f docker-compose.prod.yaml logs -f

# Stop everything
docker compose -f docker-compose.prod.yaml down
```

## Architecture Overview

```
┌─────────────────────────────────────┐
│  Host Machine                       │
│  ┌─────────────────────────────┐   │
│  │  Ollama (port 11434)        │   │
│  │  - nomic-embed-text         │   │
│  │  - llama3.2:1b              │   │
│  └─────────────────────────────┘   │
└───────────────┬─────────────────────┘
                │ host.docker.internal
                │
┌───────────────▼─────────────────────┐
│  Docker Network                     │
│  ┌───────────────────────────────┐ │
│  │  Frontend (Next.js)           │ │
│  │  - Port: 3000                 │ │
│  │  - Size: ~150 MB              │ │
│  └───────────┬───────────────────┘ │
│              │ HTTP                 │
│  ┌───────────▼───────────────────┐ │
│  │  Backend (Rust/Axum)          │ │
│  │  - Port: 3001                 │ │
│  │  - Size: ~200 MB              │ │
│  │  - Auto migrations            │ │
│  └───────────┬───────────────────┘ │
│              │                      │
│  ┌───────────▼───────────────────┐ │
│  │  PostgreSQL 18                │ │
│  │  - Port: 5432                 │ │
│  │  - Volume: persistent         │ │
│  └───────────────────────────────┘ │
└─────────────────────────────────────┘
```

## Expected Results

### Image Sizes

```bash
$ docker images
REPOSITORY                SIZE
utprotocol-backend        200 MB
utprotocol-frontend       150 MB
postgres:18-alpine        240 MB
```

### Build Times

- Backend (first build): ~5-10 minutes
- Backend (incremental): ~1-2 minutes
- Frontend (first build): ~2-3 minutes
- Frontend (incremental): ~30 seconds

### Startup Sequence

1. PostgreSQL starts (5-10 seconds)
2. Backend waits for PostgreSQL health check
3. Backend runs migrations (2-3 seconds)
4. Backend starts server (~1 second)
5. Frontend starts (~2 seconds)

**Total startup time**: ~15-20 seconds

## Testing the Setup

### 1. Check all services are running

```bash
docker compose -f docker-compose.dev.yaml ps

# Expected output:
# NAME                          STATUS      PORTS
# utprotocol-postgres-dev       healthy     0.0.0.0:5432->5432/tcp
# utprotocol-backend-dev        healthy     0.0.0.0:3001->3001/tcp
# utprotocol-frontend-dev       healthy     0.0.0.0:3000->3000/tcp
```

### 2. Test backend health

```bash
curl http://localhost:3001/health

# Expected output:
# {"status":"ok","ollama_connected":true}
```

### 3. Test frontend

```bash
open http://localhost:3000
# Should load the UTP chat interface
```

### 4. Test database

```bash
docker compose -f docker-compose.dev.yaml exec postgres \
  psql -U utprotocol -c "SELECT COUNT(*) FROM conversations;"

# Should return a count (0 or more)
```

### 5. Test Ollama integration

```bash
# Upload a document via the UI or API
curl -X POST http://localhost:3001/documents/upload \
  -F "title=Test Doc" \
  -F "file=@test.pdf"

# Check backend logs for embedding generation
docker compose -f docker-compose.dev.yaml logs backend | grep "embedding"
```

## Next Steps

1. **Read DOCKER.md** for detailed usage and troubleshooting
2. **Configure .env** with your specific settings
3. **Add Groq API key** (optional) for cloud models
4. **Set up monitoring** (Prometheus, Grafana)
5. **Configure reverse proxy** (Nginx, Traefik) for HTTPS
6. **Implement CI/CD** pipeline for automated deployments

## Support

For issues or questions:
1. Check DOCKER.md troubleshooting section
2. Review container logs: `docker compose logs -f`
3. Check GitHub issues
4. Consult Docker documentation

---

**Status**: ✅ Complete - All Docker files created and tested
**Last Updated**: 2025-11-02
