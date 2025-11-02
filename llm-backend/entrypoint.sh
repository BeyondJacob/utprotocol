#!/bin/bash
set -e

echo "🚀 Starting UTP Backend entrypoint..."

# Source Groq API key if file exists
if [ -f "/app/.env.groq" ]; then
    echo "📝 Loading Groq API configuration..."
    set -a
    source /app/.env.groq
    set +a
    echo "✅ Groq configuration loaded"
fi

# Wait for PostgreSQL to be ready
echo "⏳ Waiting for PostgreSQL to be ready..."
max_attempts=30
attempt=0

until psql "$DATABASE_URL" -c '\q' 2>/dev/null; do
    attempt=$((attempt + 1))
    if [ $attempt -eq $max_attempts ]; then
        echo "❌ PostgreSQL is not available after $max_attempts attempts"
        exit 1
    fi
    echo "   Attempt $attempt/$max_attempts: PostgreSQL is unavailable - sleeping"
    sleep 2
done

echo "✅ PostgreSQL is ready!"

# Run database migrations
echo "📦 Running database migrations..."
for migration in /app/migrations/*.sql; do
    if [ -f "$migration" ]; then
        echo "   Running $(basename "$migration")..."
        psql "$DATABASE_URL" -f "$migration" 2>&1 | grep -v "already exists" || true
    fi
done
echo "✅ Migrations completed!"

# Check Ollama connectivity (non-blocking)
OLLAMA_URL="${OLLAMA_URL:-http://host.docker.internal:11434}"
echo "🔍 Checking Ollama connectivity at $OLLAMA_URL..."
if curl -s -o /dev/null -w "%{http_code}" "$OLLAMA_URL/api/tags" | grep -q "200"; then
    echo "✅ Ollama is available"
else
    echo "⚠️  Warning: Ollama not reachable at $OLLAMA_URL"
    echo "   The backend will start but embedding generation may fail"
    echo "   Make sure Ollama is running on the host machine"
fi

# Start the backend server
echo "🚀 Starting UTP Backend server..."
exec /app/llm-backend
