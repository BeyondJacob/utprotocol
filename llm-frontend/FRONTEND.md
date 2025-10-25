# LLM Frontend - Next.js Implementation

## Overview

Modern Next.js 15 frontend with TypeScript and shadcn/ui, featuring:
- **Multi-Window Chat Interface**: Side-by-side comparison (up to 3 windows)
- **UTP Integration**: Real-time display of compression and caching metrics
- **Model Management**: Download, delete, and switch between models
- **Conversation Persistence**: Full chat history with metadata
- **Responsive Design**: Mobile-first approach with Tailwind CSS

## Technology Stack

- **Framework**: Next.js 15.5.5 with Turbopack
- **React**: 19.1.0
- **UI Components**: shadcn/ui + Radix UI primitives
- **Styling**: Tailwind CSS 4
- **Icons**: lucide-react
- **Language**: TypeScript 5

## Project Structure

```
llm-frontend/
├── package.json              # Dependencies
├── FRONTEND.md              # This file
├── tsconfig.json            # TypeScript config
├── next.config.ts           # Next.js config
├── components.json          # shadcn/ui config
├── tailwind.config.ts       # Tailwind config
├── app/
│   ├── layout.tsx           # Root layout
│   ├── page.tsx             # Main page (renders MultiChatInterface)
│   ├── globals.css          # Global styles
│   └── favicon.ico
├── components/
│   ├── multi-chat-interface.tsx  # Main orchestrator (3-window layout)
│   ├── chat-window.tsx          # Individual chat window
│   ├── chat-sidebar.tsx         # Conversation history sidebar
│   ├── message-list.tsx         # Message display with UTP badges
│   ├── chat-input.tsx           # Message input field
│   ├── model-selector.tsx       # Model dropdown
│   ├── utp-stats-panel.tsx      # UTP performance dashboard (NEW)
│   └── ui/                      # shadcn/ui components
│       ├── button.tsx
│       ├── card.tsx
│       ├── input.tsx
│       ├── scroll-area.tsx
│       ├── select.tsx
│       ├── sheet.tsx
│       └── ...
├── lib/
│   └── utils.ts             # Utility functions (cn helper)
└── public/
    └── groq-logo.svg        # Provider icon
```

## Component Hierarchy

```
page.tsx
└── MultiChatInterface (Main layout manager)
    ├── Header (App title + layout controls)
    ├── ChatSidebar (Conversation list)
    │   ├── New Conversation button
    │   └── Conversation items
    ├── ChatWindow[] (1-3 windows)
    │   ├── Window Header (Model selector, close, maximize)
    │   ├── MessageList (Messages with UTP badges)
    │   │   ├── User messages (right-aligned, blue)
    │   │   └── Assistant messages (left-aligned, gray)
    │   │       └── UtpMetricsBadge (compression, cache hit, latency)
    │   └── ChatInput (Text input + send button)
    └── UtpStatsPanel (Performance dashboard) [NEW]
        ├── Cache Performance card
        ├── Compression Stats card
        └── Speed Comparison card
```

## Key Components

### 1. MultiChatInterface (`multi-chat-interface.tsx`)

**Main orchestrator** managing multiple chat windows and layout modes.

**Features:**
- Up to 3 simultaneous chat windows
- Layout modes: Single, Dual, Triple
- Window maximization/restoration
- Conversation loading and management
- Responsive design (collapses on mobile)

**State:**
```typescript
type ChatWindowState = {
  id: string;
  conversationId: number | null;
};

type LayoutMode = "single" | "dual" | "triple";
```

**Key Methods:**
- `addWindow()`: Create new chat window (max 3)
- `removeWindow(id)`: Close a window
- `loadConversationInWindow(convId, windowId)`: Load conversation
- `toggleMaximize(windowId)`: Maximize/restore window
- `createNewConversation()`: Create new conversation with UTP settings

### 2. ChatWindow (`chat-window.tsx`)

**Individual chat window** with model selection and message display.

**Features:**
- Model selector dropdown
- Message history display
- Real-time latency tracking
- Token usage and cost calculation (for cloud models)
- Window controls (close, maximize)

**Props:**
```typescript
interface ChatWindowProps {
  windowId: string;
  conversationId: number | null;
  onClose: () => void;
  availableModels: ModelInfo[];
  onConversationChange?: (id: number) => void;
  isMaximized?: boolean;
  onToggleMaximize?: () => void;
}
```

**Types:**
```typescript
export type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string;
  latency?: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  tokens_per_second?: number;
  total_tokens?: number;
};

export type ModelInfo = {
  name: string;
  display_name: string;
  provider: string;
  is_local: boolean;
  downloaded: boolean;
  pricing?: ModelPricing;
};
```

### 3. MessageList (`message-list.tsx`)

**Message display** with UTP performance badges.

**Features:**
- User messages: right-aligned, blue background
- Assistant messages: left-aligned, gray background
- Model name and latency display
- Token usage and cost breakdown (cloud models)
- **UTP metadata badges** (NEW):
  - Cache hit indicator (green lightning bolt)
  - Compression ratio display
  - Size comparison (original → compressed)
  - Precision level (F32/F16/Int8/Int4)

**UTP Badge Example:**
```tsx
{message.utp_metadata && (
  <div className="mt-2 flex flex-wrap gap-2 text-xs">
    {message.utp_metadata.used_utp ? (
      <>
        <Badge variant="outline" className="bg-blue-50">
          UTP Mode
        </Badge>
        {message.utp_metadata.cache_hit && (
          <Badge variant="outline" className="bg-green-50 text-green-700">
            ⚡ Cache Hit
          </Badge>
        )}
        <Badge variant="outline">
          {message.utp_metadata.compression_ratio.toFixed(2)}x compression
        </Badge>
        <Badge variant="outline">
          {message.utp_metadata.compressed_size} bytes
          (saved {message.utp_metadata.original_size - message.utp_metadata.compressed_size})
        </Badge>
        <Badge variant="outline">
          {message.utp_metadata.precision_used}
        </Badge>
      </>
    ) : (
      <Badge variant="outline" className="bg-gray-50">
        Traditional Mode
      </Badge>
    )}
  </div>
)}
```

### 4. ChatSidebar (`chat-sidebar.tsx`)

**Conversation history** sidebar with create/delete actions.

**Features:**
- List of all conversations (most recent first)
- Active conversation highlighting
- Create new conversation button
- Delete conversation action
- Responsive (sheet on mobile, persistent on desktop)

**Props:**
```typescript
export type Conversation = {
  id: number;
  title: string;
  model: string;
  utp_enabled: boolean;  // NEW
  created_at: string;
  updated_at: string;
};
```

### 5. ModelSelector (`model-selector.tsx`)

**Model dropdown** with download/delete actions for local models.

**Features:**
- Provider icons (Groq logo, Network icon for local, Cloud for others)
- Download status indicators (✅ downloaded, ❌ not downloaded)
- Hover actions:
  - Downloaded models: Trash icon to delete
  - Not downloaded: Download icon to pull
- Model pricing info for cloud models
- 320px width to fit long display names

**Model Display Format:**
```
GPT-OSS 20B (Local)     [Ollama icon] [✅]
GPT-OSS 20B (Groq)      [Groq logo]   [✅]
Llama 3.3 70B (Groq)    [Groq logo]   [✅] $0.59/$0.79
```

### 6. UtpStatsPanel (`utp-stats-panel.tsx`) [NEW]

**Real-time performance dashboard** comparing Traditional vs UTP.

**Features:**
- Fetches `/utp/stats` every 5 seconds
- Three performance cards:
  1. **Cache Performance**: Hit rate, hits vs misses, total savings
  2. **Compression Stats**: Average ratio, size before/after, bytes saved
  3. **Speed Comparison**: Speedup factor, latency comparison, improvement percentage

**Component Structure:**
```tsx
export function UtpStatsPanel() {
  const [stats, setStats] = useState<UtpStats | null>(null);

  useEffect(() => {
    const fetchStats = async () => {
      const response = await fetch("http://localhost:3001/utp/stats");
      const data = await response.json();
      setStats(data);
    };

    fetchStats();
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, []);

  if (!stats) return <div>Loading...</div>;

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4 p-4">
      <Card>
        <CardHeader>Cache Performance</CardHeader>
        <CardContent>
          <Progress value={stats.cache_stats.hit_rate * 100} />
          <div>Hits: {stats.cache_stats.hits}</div>
          <div>Misses: {stats.cache_stats.misses}</div>
          <div>Savings: {stats.cache_stats.total_savings_ms}ms</div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>Compression Stats</CardHeader>
        <CardContent>
          <div className="text-3xl font-bold">
            {stats.performance_metrics.avg_compression_ratio}x
          </div>
          <div>{stats.performance_metrics.avg_size_traditional_bytes}B → {stats.performance_metrics.avg_size_utp_bytes}B</div>
          <div>Saved: {stats.performance_metrics.size_reduction_percent}%</div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>Speed Comparison</CardHeader>
        <CardContent>
          <div className="text-3xl font-bold">
            {stats.performance_metrics.speedup_factor}x faster
          </div>
          <div>Traditional: {stats.performance_metrics.avg_latency_traditional_us / 1000}ms</div>
          <div>UTP: {stats.performance_metrics.avg_latency_utp_us / 1000}ms</div>
        </CardContent>
      </Card>
    </div>
  );
}
```

## API Integration

### Chat Request (UTP-Enabled)
```typescript
const sendMessage = async (message: string) => {
  const response = await fetch("http://localhost:3001/chat", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      model: currentModel,
      message: message,
      conversation_id: conversationId,
      use_utp: isUtpWindow,  // NEW: Enable UTP
    }),
  });

  const data = await response.json();

  // data.utp_metadata contains performance metrics
  if (data.utp_metadata) {
    console.log("UTP Metrics:", {
      cacheHit: data.utp_metadata.cache_hit,
      compression: data.utp_metadata.compression_ratio,
      latency: data.utp_metadata.latency_us,
    });
  }
};
```

### Create Conversation (UTP-Aware)
```typescript
const createNewConversation = async (useUtp: boolean = false) => {
  const response = await fetch("http://localhost:3001/conversations", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      title: "New Chat",
      model: "gpt-oss:20b",
      utp_enabled: useUtp,  // NEW: Set UTP flag
    }),
  });

  const newConversation = await response.json();
  // Conversation will use UTP for all messages
};
```

## Styling with Tailwind

### Theme Colors
```css
/* globals.css */
:root {
  --background: 0 0% 100%;
  --foreground: 222.2 84% 4.9%;
  --card: 0 0% 100%;
  --card-foreground: 222.2 84% 4.9%;
  --primary: 222.2 47.4% 11.2%;
  --primary-foreground: 210 40% 98%;
  /* ... more CSS variables ... */
}
```

### Component Styling Examples
```tsx
// User message (right-aligned, blue)
<div className="flex justify-end">
  <div className="bg-blue-500 text-white rounded-lg px-4 py-2 max-w-[80%]">
    {message.content}
  </div>
</div>

// Assistant message (left-aligned, gray)
<div className="flex justify-start">
  <div className="bg-gray-100 text-gray-900 rounded-lg px-4 py-2 max-w-[80%]">
    {message.content}
  </div>
</div>

// UTP Cache Hit badge (green)
<Badge variant="outline" className="bg-green-50 text-green-700">
  ⚡ Cache Hit
</Badge>

// UTP Mode badge (blue)
<Badge variant="outline" className="bg-blue-50 text-blue-700">
  UTP Mode
</Badge>
```

## Development

### Setup
```bash
# Install dependencies
npm install

# Run development server
npm run dev

# Open browser
open http://localhost:3000
```

### Build
```bash
# Production build
npm run build

# Start production server
npm start
```

### Linting
```bash
# Run ESLint
npm run lint

# Fix auto-fixable issues
npm run lint -- --fix
```

## Environment Variables

```env
# .env.local (optional)
NEXT_PUBLIC_API_URL=http://localhost:3001
```

## Responsive Design

### Breakpoints
- **Mobile**: < 640px (single column)
- **Tablet**: 640px - 1024px (dual column possible)
- **Desktop**: > 1024px (triple column available)

### Layout Adaptation
```tsx
// Single column on mobile, dual on tablet, triple on desktop
<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
  {windows.map(window => <ChatWindow key={window.id} {...window} />)}
</div>
```

## UTP Integration Checklist

- [x] Update `ChatRequest` type to include `use_utp` field
- [x] Update `ChatResponse` type to include `utp_metadata` field
- [x] Update `Conversation` type to include `utp_enabled` field
- [x] Update `Message` type to include `utp_metadata` field
- [ ] Modify `ChatWindow` to pass `use_utp` based on conversation settings
- [ ] Add UTP metadata badges to `MessageList`
- [ ] Create `UtpStatsPanel` component
- [ ] Add UTP toggle to conversation creation
- [ ] Add window labels ("Traditional" vs "UTP")
- [ ] Integrate stats panel into main layout

## Testing

### Manual Testing
```bash
# 1. Start backend
cd llm-backend && cargo run

# 2. Start frontend
cd llm-frontend && npm run dev

# 3. Test UTP flow
# - Create new conversation with UTP enabled
# - Send message
# - Verify UTP badges appear
# - Send same message again
# - Verify cache hit badge appears
# - Check stats panel for metrics
```

### Key Test Cases
1. **Traditional Mode**: No UTP badges, full latency
2. **UTP Cache Miss**: UTP badge, compression shown, normal latency
3. **UTP Cache Hit**: Cache hit badge, <1ms latency
4. **Side-by-side**: Traditional vs UTP in dual window mode
5. **Stats Panel**: Real-time updates every 5 seconds

## Troubleshooting

### API Connection Issues
```typescript
// Check backend is running
fetch("http://localhost:3001/health")
  .then(res => res.json())
  .then(data => console.log("Backend status:", data))
  .catch(err => console.error("Backend not responding:", err));
```

### CORS Errors
Backend is configured to allow all origins:
```rust
// llm-backend/src/main.rs
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::DELETE])
    .allow_headers(Any);
```

### UTP Metadata Not Showing
```typescript
// Debug message data
console.log("Message:", message);
console.log("UTP Metadata:", message.utp_metadata);

// Check if backend returned metadata
// Should have: used_utp, cache_hit, compression_ratio, etc.
```

## Performance Optimization

### Code Splitting
```tsx
// Lazy load heavy components
import dynamic from 'next/dynamic';

const UtpStatsPanel = dynamic(() => import('./utp-stats-panel'), {
  loading: () => <div>Loading stats...</div>,
});
```

### Memoization
```tsx
import { memo, useMemo } from 'react';

export const MessageList = memo(({ messages }) => {
  const sortedMessages = useMemo(
    () => messages.sort((a, b) => a.created_at - b.created_at),
    [messages]
  );

  return <div>{sortedMessages.map(renderMessage)}</div>;
});
```

### Virtual Scrolling
For long message lists (100+ messages), consider react-window or react-virtual.

## Future Enhancements

- [ ] Message search/filter
- [ ] Export conversation to Markdown/JSON
- [ ] Drag-and-drop window rearrangement
- [ ] Custom UTP precision selection per conversation
- [ ] Real-time UTP metrics streaming (WebSocket)
- [ ] Dark mode support
- [ ] Message editing/regeneration
- [ ] Code syntax highlighting
- [ ] Image/file attachments
- [ ] Voice input/output

## References

- [Next.js Documentation](https://nextjs.org/docs)
- [shadcn/ui Components](https://ui.shadcn.com)
- [Radix UI Primitives](https://www.radix-ui.com)
- [Tailwind CSS](https://tailwindcss.com)
- [Lucide Icons](https://lucide.dev)
