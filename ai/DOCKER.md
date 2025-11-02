# Docker Setup Guide

This guide explains how to run the Universal Thought Protocol (UTP) application using Docker and Docker Compose.

## Prerequisites

1. **Docker** (v24.0+) and **Docker Compose** (v2.0+)
   ```bash
   docker --version
   docker compose version
   ```

2. **Ollama** running on your host machine (required for embeddings)
   ```bash
   ollama --version
   ollama pull nomic-embed-text  # Required for embeddings
   ollama pull llama3.2:1b       # Optional: For testing
   ```

3. **Environment files** (copy from examples)
   ```bash
   cp .env.example .env
   cp .env.groq.example .env.groq  # Optional: Only if using Groq
   ```

## Quick Start (Development)

```bash
# 1. Build and start all services
docker compose -f docker-compose.dev.yaml up --build

# 2. Open your browser
open http://localhost:3000

# 3. Stop services
docker compose -f docker-compose.dev.yaml down
```

That's it! The backend will automatically:
- Wait for PostgreSQL to be ready
- Run database migrations
- Connect to your host machine's Ollama instance
- Start the server on port 3001

## Quick Start (Production)

```bash
# 1. Create production environment file
cp .env.example .env
# Edit .env with your production values

# 2. (Optional) Add Groq API key
cp .env.groq.example llm-backend/.env.groq
# Edit llm-backend/.env.groq with your API key

# 3. Build and start all services
docker compose -f docker-compose.prod.yaml up --build -d

# 4. Check logs
docker compose -f docker-compose.prod.yaml logs -f

# 5. Stop services
docker compose -f docker-compose.prod.yaml down
```

## Architecture

The Docker setup consists of three services:

```
┌─────────────────────────────────────┐
│  Frontend (Next.js)                 │
│  Port: 3000                         │
│  Image: node:20-alpine (multi-stage)│
└──────────────┬──────────────────────┘
               │ HTTP
               ▼
┌─────────────────────────────────────┐
│  Backend (Rust/Axum)                │
│  Port: 3001                         │
│  Image: rust:1.83 → debian:bookworm │
└──────┬──────────────────────┬───────┘
       │                      │
       │ PostgreSQL           │ Ollama
       ▼                      ▼
┌──────────────┐    ┌─────────────────┐
│  PostgreSQL  │    │  Host Machine   │
│  Port: 5432  │    │  Port: 11434    │
│  postgres:17 │    │  (via host.     │
│              │    │   docker.       │
│              │    │   internal)     │
└──────────────┘    └─────────────────┘
```

## Docker Images

### Backend (Multi-stage Build)

**Stage 1: Builder** (rust:1.91-slim)
- Size: ~1.5 GB
- Installs build dependencies (libssl-dev, libpq-dev)
- Compiles Rust binary with `cargo build --release`

**Stage 2: Runtime** (debian:bookworm-slim)
- Size: ~200 MB (final image)
- Only runtime dependencies (libssl3, libpq5)
- Copies compiled binary from builder stage
- Runs migrations via entrypoint script
- Non-root user (utprotocol)

### Frontend (Multi-stage Build)

**Stage 1: Dependencies** (node:20-alpine)
- Installs npm dependencies

**Stage 2: Builder** (node:20-alpine)
- Builds Next.js application with standalone output

**Stage 3: Runtime** (node:20-alpine)
- Size: ~150 MB (final image)
- Only production dependencies
- Standalone Next.js build
- Non-root user (nextjs)

### PostgreSQL

- Official PostgreSQL 18 Alpine image (~240 MB)
- Named volume for data persistence
- Health checks enabled

## Environment Variables

### Required Variables

```bash
# Database (automatically set in docker-compose)
DATABASE_URL=postgres://utprotocol:utprotocol_password@postgres:5432/utprotocol

# Ollama (connects to host machine)
OLLAMA_URL=http://host.docker.internal:11434
```

### Optional Variables

```bash
# Groq API key (for cloud models)
GROQ_API_KEY=your_api_key_here

# Logging level
RUST_LOG=info  # Options: error, warn, info, debug, trace

# Custom ports (production only)
POSTGRES_PORT=5432
BACKEND_PORT=3001
FRONTEND_PORT=3000

# Frontend API URL (if using reverse proxy)
NEXT_PUBLIC_API_URL=http://localhost:3001
```

## Docker Compose Files

### `docker-compose.dev.yaml` (Development)

Features:
- Debug logging enabled (`RUST_LOG=debug`)
- No resource limits
- Optional source code mounting for hot reload
- Development-friendly healthchecks

Usage:
```bash
docker compose -f docker-compose.dev.yaml up
docker compose -f docker-compose.dev.yaml logs backend
docker compose -f docker-compose.dev.yaml exec backend /bin/bash
```

### `docker-compose.prod.yaml` (Production)

Features:
- Production logging (`RUST_LOG=info`)
- Resource limits (CPU/memory)
- Log rotation (10MB max, 3 files)
- Environment variable overrides
- Always restart policy
- Backup volume for PostgreSQL

Usage:
```bash
docker compose -f docker-compose.prod.yaml up -d
docker compose -f docker-compose.prod.yaml ps
docker compose -f docker-compose.prod.yaml restart backend
```

## Common Tasks

### View Logs

```bash
# All services
docker compose -f docker-compose.dev.yaml logs -f

# Specific service
docker compose -f docker-compose.dev.yaml logs -f backend

# Last 100 lines
docker compose -f docker-compose.dev.yaml logs --tail=100 backend
```

### Execute Commands in Containers

```bash
# Access backend shell
docker compose -f docker-compose.dev.yaml exec backend /bin/bash

# Access PostgreSQL
docker compose -f docker-compose.dev.yaml exec postgres psql -U utprotocol

# Run migrations manually
docker compose -f docker-compose.dev.yaml exec backend \
  psql "$DATABASE_URL" -f /app/migrations/007_create_vector_storage.sql
```

### Rebuild After Code Changes

```bash
# Rebuild specific service
docker compose -f docker-compose.dev.yaml up --build backend

# Rebuild all services
docker compose -f docker-compose.dev.yaml up --build

# Force rebuild (no cache)
docker compose -f docker-compose.dev.yaml build --no-cache
```

### Database Management

```bash
# Backup database
docker compose -f docker-compose.prod.yaml exec postgres \
  pg_dump -U utprotocol utprotocol > backup_$(date +%Y%m%d).sql

# Restore database
cat backup_20250102.sql | docker compose -f docker-compose.prod.yaml exec -T postgres \
  psql -U utprotocol utprotocol

# Access PostgreSQL shell
docker compose -f docker-compose.prod.yaml exec postgres \
  psql -U utprotocol utprotocol
```

### Clean Up

```bash
# Stop and remove containers (keeps volumes)
docker compose -f docker-compose.dev.yaml down

# Remove containers and volumes (DESTRUCTIVE)
docker compose -f docker-compose.dev.yaml down -v

# Remove images
docker compose -f docker-compose.dev.yaml down --rmi all

# Full cleanup (containers, volumes, images, orphans)
docker compose -f docker-compose.dev.yaml down -v --rmi all --remove-orphans
```

## Troubleshooting

### Backend can't connect to Ollama

**Problem**: `⚠️ Warning: Ollama not reachable`

**Solution**:
1. Ensure Ollama is running on host:
   ```bash
   ollama list
   curl http://localhost:11434/api/tags
   ```

2. Check Docker host gateway:
   ```bash
   docker compose -f docker-compose.dev.yaml exec backend \
     curl http://host.docker.internal:11434/api/tags
   ```

3. On Linux, you may need to use `host.docker.internal` or `172.17.0.1`:
   ```yaml
   environment:
     OLLAMA_URL: http://172.17.0.1:11434
   ```

### PostgreSQL connection refused

**Problem**: `PostgreSQL is not available`

**Solution**:
1. Wait for PostgreSQL to be healthy:
   ```bash
   docker compose -f docker-compose.dev.yaml ps
   # Look for "healthy" status
   ```

2. Check logs:
   ```bash
   docker compose -f docker-compose.dev.yaml logs postgres
   ```

3. Test connection:
   ```bash
   docker compose -f docker-compose.dev.yaml exec postgres \
     pg_isready -U utprotocol
   ```

### Migration errors

**Problem**: `relation already exists`

**Solution**: This is expected if migrations already ran. The entrypoint script filters these warnings:
```bash
# Check migration status
docker compose -f docker-compose.dev.yaml exec postgres \
  psql -U utprotocol -d utprotocol -c "\dt"
```

### Port conflicts

**Problem**: `Bind for 0.0.0.0:3000 failed: port is already allocated`

**Solution**:
1. Stop conflicting services:
   ```bash
   lsof -ti:3000 | xargs kill -9
   ```

2. Or change ports in docker-compose:
   ```yaml
   ports:
     - "3002:3000"  # Host:Container
   ```

### Out of disk space

**Problem**: `no space left on device`

**Solution**:
```bash
# Remove unused Docker resources
docker system prune -a --volumes

# Check disk usage
docker system df
```

### Frontend can't reach backend

**Problem**: API calls fail from browser

**Solution**:
1. Verify backend is running:
   ```bash
   curl http://localhost:3001/health
   ```

2. Check browser console for CORS errors

3. Update frontend environment:
   ```yaml
   environment:
     NEXT_PUBLIC_API_URL: http://localhost:3001
   ```

## Performance Tuning

### Production Optimizations

1. **Resource Limits** (already configured in prod compose):
   ```yaml
   deploy:
     resources:
       limits:
         cpus: '4'
         memory: 4G
   ```

2. **PostgreSQL Tuning**:
   ```yaml
   postgres:
     command:
       - postgres
       - -c
       - max_connections=100
       - -c
       - shared_buffers=256MB
   ```

3. **Backend Thread Pool**:
   ```bash
   environment:
     TOKIO_WORKER_THREADS: 4
   ```

### Build Optimizations

1. **Cache Dependencies**:
   - Backend: Cargo dependencies are cached by copying `Cargo.toml` first
   - Frontend: npm dependencies are cached in separate stage

2. **Build Time**:
   - Backend: ~5-10 minutes (first build), ~1-2 minutes (incremental)
   - Frontend: ~2-3 minutes (first build), ~30 seconds (incremental)

3. **Image Sizes**:
   - Backend: ~200 MB (vs ~1.5 GB builder)
   - Frontend: ~150 MB (vs ~800 MB builder)

## Security Considerations

1. **Non-root Users**: Both containers run as non-root users (utprotocol, nextjs)

2. **Secret Management**:
   - Never commit `.env` or `.env.groq` files
   - Use Docker secrets in production:
     ```bash
     docker secret create groq_api_key .env.groq
     ```

3. **Network Isolation**: Services communicate via internal Docker network

4. **Resource Limits**: Production compose includes CPU/memory limits

5. **Read-only Filesystem** (optional):
   ```yaml
   backend:
     security_opt:
       - no-new-privileges:true
     read_only: true
     tmpfs:
       - /tmp
   ```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build Docker Images

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build backend
        run: docker build -t utprotocol-backend:${{ github.sha }} ./llm-backend

      - name: Build frontend
        run: docker build -t utprotocol-frontend:${{ github.sha }} ./llm-frontend

      - name: Push to registry
        run: |
          docker tag utprotocol-backend:${{ github.sha }} registry.example.com/backend:latest
          docker push registry.example.com/backend:latest
```

## Next Steps

1. **Deploy to Cloud**: Use docker-compose.prod.yaml with cloud PostgreSQL (AWS RDS, GCP Cloud SQL)
2. **Add Reverse Proxy**: Nginx or Traefik for HTTPS and load balancing
3. **Monitoring**: Add Prometheus + Grafana for metrics
4. **Logging**: Centralized logging with ELK stack or Loki

## References

- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Multi-stage Builds](https://docs.docker.com/build/building/multi-stage/)
- [Next.js Standalone Output](https://nextjs.org/docs/advanced-features/output-file-tracing)
- [PostgreSQL Docker Image](https://hub.docker.com/_/postgres)
