#!/bin/bash
set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "Starting llama.cpp servers..."

# Check if llama.cpp exists and build it
if [ ! -d "llama.cpp" ]; then
    echo "Cloning llama.cpp..."
    git clone https://github.com/ggerganov/llama.cpp
fi

cd llama.cpp

# Build with CMake if not already built
if [ ! -f "build/bin/llama-server" ]; then
    echo "Building llama.cpp with Metal support..."
    cmake -B build -DGGML_METAL=ON
    cmake --build build --config Release -j 8
fi

cd ..

mkdir -p logs

# Start GPT-OSS server
echo "Starting GPT-OSS on port 8080..."
./llama.cpp/build/bin/llama-server \
    -m models/gpt-oss-20b/gpt-oss-20b-mxfp4.gguf \
    -c 8192 \
    --host 127.0.0.1 \
    --port 8080 \
    -ngl 999 \
    > logs/gpt-oss.log 2>&1 &

echo $! > logs/gpt-oss.pid
echo "GPT-OSS PID: $(cat logs/gpt-oss.pid)"

echo ""
echo "Server started: http://localhost:8080"
echo "View logs: tail -f logs/gpt-oss.log"
echo "Stop: ./scripts/stop_local.sh"
