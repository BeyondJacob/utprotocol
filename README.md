# Universal Thought Protocol (UTP)

A powerful full-stack LLM chat application with Rust backend and Next.js frontend, featuring one-click model management and multi-model support. Run OpenAI's gpt-oss models (20B and 120B) and other LLMs locally via Ollama.

## Architecture

```
┌─────────────────────────────────┐
│   Next.js Frontend              │
│   - shadcn/ui chat interface    │
│   - Model selector with status  │
│   - One-click download/delete   │
│   - Message history display     │
└──────────┬──────────────────────┘
           │ HTTP (port 3000 → 3001)
           │
┌──────────▼──────────────────────┐
│   Rust Backend (Axum)           │
│   - Direct Ollama API calls     │
│   - POST /chat                  │
│   - GET /models (with status)   │
│   - POST /download-model        │
│   - POST /delete-model          │
│   - POST /switch-model          │
│   - GET /health                 │
└──────────┬──────────────────────┘
           │
┌──────────▼──────────────────────┐
│   Ollama (localhost:11434)      │
│   - gpt-oss:20b                 │
│   - gpt-oss:120b                │
│   - llama3.2:1b                 │
│   - llama3.2:3b                 │
└─────────────────────────────────┘
```

## Tech Stack

- **Backend**: Rust with Axum web framework
- **Frontend**: Next.js 15 with TypeScript and Tailwind CSS
- **UI Components**: shadcn/ui
- **Model Hosting**: Ollama
- **Communication**: REST API

## Prerequisites

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** (v18 or higher)
   ```bash
   # Install via nvm, homebrew, or official installer
   ```

3. **Ollama**
   ```bash
   brew install ollama
   brew services start ollama
   ```

## Setup Instructions

### 1. Install Ollama Models

Pull the OpenAI OSS models (this may take a while as they're large):

```bash
# Pull gpt-oss models
ollama pull gpt-oss:20b   # ~13GB download
ollama pull gpt-oss:120b  # ~75GB download

# Verify models are available
ollama list
```

**Note**: For testing purposes, the app is also configured to work with smaller models like `llama3.2:1b` and `llama3.2:3b`.

### 2. Start the Rust Backend

```bash
cd llm-backend
cargo run
```

The backend will start on `http://127.0.0.1:3001`

You should see:
```
🚀 Server running on http://127.0.0.1:3001
```

### 3. Start the Next.js Frontend

In a new terminal:

```bash
cd llm-frontend
npm install  # Only needed first time
npm run dev
```

The frontend will start on `http://localhost:3000`

### 4. Open the Application

Visit [http://localhost:3000](http://localhost:3000) in your browser.

## Usage

### Model Management

1. **View Model Status**: Open the model selector dropdown to see all available models with status indicators:
   - ✅ Green checkmark = Model is downloaded and ready to use
   - ❌ Red X = Model is not downloaded

2. **Download Models**:
   - Hover over a model with a red X
   - Click the download icon (⬇️) that appears
   - Model will download in the background (may take several minutes for large models)
   - Status will update to checkmark when complete

3. **Delete Models**:
   - Hover over a downloaded model (green checkmark)
   - Click the trash icon (🗑️) that appears
   - Model will be removed from your system to free up space

### Chat Features

4. **Select Active Model**: Click on any downloaded model in the dropdown to switch to it

5. **Send Messages**: Type your message in the input field at the bottom and press Enter or click "Send"

6. **View Responses**: Messages will appear in the chat interface with:
   - User messages aligned to the right (blue background)
   - Assistant responses aligned to the left (gray background)
   - Model name and latency shown for each assistant response

7. **Switch Models Mid-Conversation**: Change models at any time - the next message will use the newly selected model

## API Endpoints

### Backend (Port 3001)

- **GET /health**
  - Returns server health and Ollama connection status
  - Response: `{"status": "ok", "ollama_connected": true}`

- **GET /models**
  - Lists all available models with download status and currently active model
  - Response: `{"models": [{"name": "gpt-oss:20b", "downloaded": true}, ...], "active_model": "gpt-oss:20b"}`

- **POST /chat**
  - Send a message to the selected model
  - Request: `{"model": "gpt-oss:20b", "message": "Hello"}`
  - Response: `{"response": "...", "model": "gpt-oss:20b", "latency_ms": 1234}`

- **POST /switch-model**
  - Change the active model
  - Request: `{"model": "gpt-oss:120b"}`
  - Response: `{"success": true, "model": "gpt-oss:120b"}`

- **POST /download-model**
  - Download a model from Ollama registry
  - Request: `{"model": "gpt-oss:20b"}`
  - Response: `{"success": true, "message": "Model gpt-oss:20b download started"}`

- **POST /delete-model**
  - Delete a downloaded model to free up disk space
  - Request: `{"model": "gpt-oss:20b"}`
  - Response: `{"success": true, "message": "Model gpt-oss:20b deleted"}`

## Testing

### Test Backend Endpoints

```bash
# Health check
curl http://localhost:3001/health

# List models
curl http://localhost:3001/models

# Send a chat message
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2:1b","message":"Hello!"}'

# Switch model
curl -X POST http://localhost:3001/switch-model \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-oss:120b"}'
```

## Project Structure

```
.
├── llm-backend/              # Rust backend
│   ├── src/
│   │   └── main.rs          # Axum server with endpoints
│   └── Cargo.toml           # Rust dependencies
│
├── llm-frontend/            # Next.js frontend
│   ├── app/
│   │   ├── page.tsx        # Main page
│   │   └── globals.css     # Global styles
│   ├── components/
│   │   ├── chat-interface.tsx    # Main chat orchestrator
│   │   ├── message-list.tsx      # Message display
│   │   ├── chat-input.tsx        # Input field
│   │   ├── model-selector.tsx    # Model dropdown
│   │   └── ui/                   # shadcn/ui components
│   ├── package.json
│   └── next.config.ts
│
└── README.md
```

## Troubleshooting

### Ollama Connection Issues

If you see `ollama_connected: false` in the health check:

```bash
# Check if Ollama is running
brew services list | grep ollama

# Start Ollama if needed
brew services start ollama

# Test Ollama directly
curl http://localhost:11434/api/tags
```

### Model Not Found

If you get errors about missing models:

```bash
# Check which models are installed
ollama list

# Pull missing models
ollama pull gpt-oss:20b
ollama pull gpt-oss:120b
```

### Port Already in Use

If port 3001 or 3000 is already in use:

```bash
# Kill process on port 3001
lsof -ti:3001 | xargs kill -9

# Kill process on port 3000
lsof -ti:3000 | xargs kill -9
```

### CORS Errors

The backend is configured to allow all origins. If you still see CORS errors:

1. Make sure both servers are running
2. Check that the frontend is calling `http://localhost:3001` (not https or different port)
3. Clear browser cache and hard reload

## Model Performance Notes

- **llama3.2:1b**: ~1-2 seconds response time, 1.3GB RAM
- **llama3.2:3b**: ~3-5 seconds response time, 2.0GB RAM
- **gpt-oss:20b**: Expected ~5-10 seconds, requires ~16GB RAM
- **gpt-oss:120b**: Expected ~30-60 seconds, requires ~80GB+ RAM

Performance will vary based on your hardware (CPU/GPU).

## Features

### Core Functionality
- ✅ Real-time chat interface with Universal Thought Protocol branding
- ✅ Multiple model support (gpt-oss, llama, and more)
- ✅ Model switching without restart
- ✅ Response latency tracking for performance monitoring

### Model Management
- ✅ **One-click model downloads** - Download any model directly from the UI
- ✅ **One-click model deletion** - Free up disk space by removing unused models
- ✅ **Real-time download status** - See which models are available vs downloaded
- ✅ **Visual status indicators** - Checkmarks and X icons show model status at a glance

### Technical Features
- ✅ Clean, modern UI with shadcn/ui components
- ✅ Full TypeScript type safety throughout the stack
- ✅ CORS-enabled REST API for flexible integration
- ✅ Comprehensive error handling and loading states
- ✅ Rust backend for high performance and safety
- ✅ Direct Ollama API integration

## Future Enhancements

- [ ] Streaming responses (SSE or WebSocket)
- [ ] Chat history persistence
- [ ] System prompts customization
- [ ] Temperature and other parameter controls
- [ ] Multi-turn conversation context
- [ ] Export chat history
- [ ] Dark mode toggle

## License

MIT

## Contributing

Feel free to open issues or submit pull requests!
