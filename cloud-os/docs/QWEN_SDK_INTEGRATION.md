# Qwen SDK Browser Client Integration

## Overview

This implementation adds a browser-compatible Qwen MCP client based on the official `@qwen-code/sdk` architecture, adapted for browser environments with OAuth authentication.

## Architecture

### Original Qwen SDK (Node.js)
```
@qwen-code/sdk
├── ProcessTransport (spawns CLI subprocess)
├── McpServer (@modelcontextprotocol/sdk)
└── Query (manages conversation)
```

### Browser Implementation
```
src/lib/qwen/
├── browser-client.ts      # Main Qwen client (like Query)
├── browser-transport.ts   # Fetch/SSE transport (replaces ProcessTransport)
├── mcp-client.ts          # MCP wrapper (like McpServer)
└── types.ts               # Protocol types
```

## Key Changes

### 1. Transport Layer
- **Node.js**: `ProcessTransport` spawns `qwen` CLI subprocess
- **Browser**: `BrowserTransport` uses `fetch()` + `EventSource` for SSE

### 2. Authentication
- **Node.js**: Reads credentials from `~/.qwen`
- **Browser**: Uses OAuth token from localStorage

### 3. Communication
```typescript
// Node.js (SDK)
const result = query({
  prompt: 'Hello',
  options: {
    pathToQwenExecutable: 'qwen',
  }
});

// Browser (Our implementation)
const client = new QwenBrowserClient({
  baseUrl: 'https://dashscope.aliyuncs.com',
  oauthToken: '...',
});
await client.connect();
await client.send('Hello');
```

## API Comparison

| Feature | Qwen SDK | Browser Client |
|---------|----------|----------------|
| Transport | Process (stdin/stdout) | Fetch + SSE |
| Auth | File-based (~/.qwen) | OAuth token |
| Session Mgmt | Automatic | Manual via API |
| MCP Support | Embedded servers | HTTP endpoints |
| Streaming | Async iterator | Async iterator |

## Usage

### Basic Chat
```typescript
import { QwenBrowserClient } from '@/lib/qwen/browser-client';

const client = new QwenBrowserClient({
  baseUrl: 'https://dashscope.aliyuncs.com',
  oauthToken: 'your-token',
  model: 'qwen-plus',
});

await client.connect();

// Send message
await client.send('Explain this code');

// Read responses
for await (const msg of client.readMessages()) {
  if (msg.type === 'assistant') {
    console.log(msg.message.content);
  }
}
```

### MCP Integration
```typescript
import { createQwenMCPClient } from '@/lib/qwen/mcp-client';

// Auto-connect with OAuth token
const token = JSON.parse(localStorage.getItem('qwen_token'));
const client = await createQwenMCPClient(token);

// Execute tools
await client.executeTool('qwen_chat', {
  message: 'Review this code',
});

// Get available tools
const tools = client.getTools();
```

## Server-Side Requirements

The browser client expects these API endpoints:

### POST /api/qwen/session
Initialize session
```json
Request: { "model": "qwen-plus", "sessionId": "optional" }
Response: { "sessionId": "abc123" }
```

### DELETE /api/qwen/session/:id
Close session

### POST /api/qwen/message
Send message
```json
Request: SDKUserMessage
```

### GET /api/qwen/stream?sessionId=:id
SSE stream for messages

## Benefits

1. **No CLI Dependency**: Doesn't require `qwen` CLI installed
2. **Pure Browser**: Works on Cloudflare Workers
3. **OAuth Native**: Built for web authentication flows
4. **Familiar API**: Matches @qwen-code/sdk patterns
5. **Smaller Bundle**: No subprocess management code

## Limitations

1. **Server Required**: Needs API endpoints (unlike local CLI)
2. **No Local Tools**: Can't run local shell commands directly
3. **SSE Required**: Server must support Server-Sent Events
4. **Token Management**: Must handle OAuth refresh manually

## Migration Path

To use the official SDK in browser:

1. Bundle CLI for browser (esbuild)
2. Use WebContainers API (Chrome only)
3. Proxy through server (current approach)

Our implementation uses option 3 for maximum compatibility.

## Files Added

```
src/lib/qwen/
├── browser-client.ts      (260 lines)
├── browser-transport.ts   (200 lines)
├── mcp-client.ts          (200 lines)
└── types.ts               (100 lines)
```

## Integration Points

1. **MCP Client Manager** (`src/lib/mcp/client.ts`)
   - Added `QwenMCPClient` support
   - Special handling for OAuth-based connection

2. **Intent Processor** (`src/lib/intent/processor.ts`)
   - Can use Qwen as LLM provider
   - Tools available via MCP

3. **Settings Panel** (`src/components/SettingsPanel.tsx`)
   - Configure Qwen OAuth
   - Set default model

## Testing

```bash
# Build
npm run build

# Test Qwen connection
# 1. Connect OAuth in Settings
# 2. Open Intent Bar
# 3. Type: "ask qwen to explain promises"
```

## Future Improvements

1. **WebSocket Transport**: Real-time bidirectional
2. **Tool Caching**: Cache available tools
3. **Auto-Reconnect**: Handle connection drops
4. **Rate Limiting**: Respect API quotas
5. **Multi-Session**: Support multiple concurrent sessions

## References

- [@qwen-code/sdk](https://github.com/QwenLM/qwen-code/tree/main/packages/sdk-typescript)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [Qwen API Docs](https://help.aliyun.com/zh/dashscope/)
