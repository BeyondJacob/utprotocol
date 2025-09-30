#!/bin/bash
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

if [ -f logs/gpt-oss.pid ]; then
    kill $(cat logs/gpt-oss.pid) 2>/dev/null || true
    rm logs/gpt-oss.pid
    echo "Stopped GPT-OSS server"
fi
