# Multi-stage Dockerfile for Rust backend
# Stage 1: Build stage using official Rust image
FROM rust:1.91-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency manifests first (for layer caching)
COPY Cargo.toml ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY config.toml ./

# Build the actual application
# Touch main.rs to force rebuild of our code
RUN touch src/main.rs && \
    cargo build --release

# Stage 2: Runtime stage using minimal Debian slim image
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    libssl3 \
    libpq5 \
    ca-certificates \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 utprotocol

# Create app directory
WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /app/target/release/llm-backend /app/llm-backend

# Copy configuration files
COPY --from=builder /app/config.toml /app/config.toml
COPY --from=builder /app/migrations /app/migrations

# Copy entrypoint script
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Copy Groq API key configuration (if exists, optional)
COPY .env.groq* /app/ 2>/dev/null || true

# Change ownership to non-root user
RUN chown -R utprotocol:utprotocol /app

# Switch to non-root user
USER utprotocol

# Expose backend port
EXPOSE 3001

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD ["/bin/sh", "-c", "command -v curl > /dev/null && curl -f http://localhost:3001/health || exit 1"]

# Use entrypoint script to run migrations then start server
ENTRYPOINT ["/app/entrypoint.sh"]
