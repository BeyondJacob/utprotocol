#!/bin/bash
set -e

echo "🔽 Downloading models..."

# Install Hugging Face CLI if not present
if ! command -v huggingface-cli &> /dev/null; then
    echo "Installing huggingface-hub..."
    pip3 install -U huggingface-hub
fi

# Download GPT-OSS 20B (MXFP4 - recommended quantization)
echo "📦 Downloading GPT-OSS 20B (MXFP4)..."
huggingface-cli download \
    ggml-org/gpt-oss-20b-GGUF \
    gpt-oss-20b-mxfp4.gguf \
    --local-dir models/gpt-oss-20b

# Download Llama 3.1 8B (Q8 quantized)
echo "📦 Downloading Llama 3.1 8B (Q8)..."
huggingface-cli download \
    bartowski/Meta-Llama-3.1-8B-Instruct-GGUF \
    Meta-Llama-3.1-8B-Instruct-Q8_0.gguf \
    --local-dir models/llama-3.1-8b

echo "✅ Models downloaded successfully!"
echo "📊 Model sizes:"
du -sh models/gpt-oss-20b/*.gguf
du -sh models/llama-3.1-8b/*.gguf

# install cmake with brew
brew install cmake