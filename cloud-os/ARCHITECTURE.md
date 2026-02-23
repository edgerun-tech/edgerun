# CloudOS AI-Centric Architecture

## Vision

Transform CloudOS from a **fixed app/window metaphor** to a **morphable, AI-driven interface** where the UI adapts to show results in the most appropriate format for each query.

## Core Principles

1. **IntentBar First** - All interactions start with natural language
2. **Morphable UI** - Results render in context-appropriate views (logs, diffs, files, emails, etc.)
3. **No Fixed Windows** - Panels appear/disappear based on query results
4. **Composable Results** - Stack multiple result views for complex queries
5. **Context Preservation** - Return to previous "workspaces" with full history
6. **Action Chaining** - Each result suggests next actions in natural language

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        IntentBar (Permanent)                     │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ "Show me the API logs from yesterday"                    │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Intent Processor → LLM → Execution Plan → MCP Tools            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Morphable Result Panels                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Log Viewer    │  │    File Grid    │  │   Code Diff     │ │
│  │   (auto-scroll) │  │   (thumbnails)  │  │   (side-by-side)│ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │    Timeline     │  │  Email Reader   │  │   JSON Tree     │ │
│  │   (vertical)    │  │  (conversation) │  │   (expandable)  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Result History / Context                      │
│  [Previous queries] [Pinned results] [Related files]            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### 1. User Query
```typescript
{
  query: "Show me what changed in the last deployment",
  timestamp: Date,
  context: {
    currentRepo: "my-app",
    currentBranch: "main",
    recentFiles: ["src/api.ts"],
  }
}
```

### 2. Intent Processing
```typescript
{
  intent: {
    verb: "show",
    target: "changes",
    modifiers: ["last", "deployment"],
    confidence: 0.9
  },
  steps: [
    { tool: "vercel_get_deployments", args: { limit: 1 } },
    { tool: "github_get_diff", args: { ref: "HEAD~1" } }
  ]
}
```

### 3. Tool Execution with UI Hints
```typescript
{
  success: true,
  data: { /* raw data */ },
  ui: {
    viewType: "code-diff",
    layout: "full",
    title: "Last Deployment Changes",
    actions: [
      { label: "Revert", intent: "revert this deployment" },
      { label: "View Logs", intent: "show deployment logs" }
    ]
  }
}
```

### 4. Result Rendering
```typescript
<ResultRenderer response={toolResponse} />
// Automatically renders CodeDiffViewer component
```

---

## View Types

| View Type | Component | Use Case |
|-----------|-----------|----------|
| `preview` | `PreviewCard` | Default, simple summaries |
| `json-tree` | `JSONTree` | API responses, config data |
| `table` | `DataTable` | Structured lists, deployments |
| `code-diff` | `CodeDiffViewer` | Git changes, PR reviews |
| `file-grid` | `FileGrid` | Search results, file browsers |
| `log-viewer` | `LogViewer` | Terminal output, error logs |
| `timeline` | `Timeline` | Event sequences, deployment history |
| `email-reader` | `EmailReader` | Gmail conversations |
| `doc-viewer` | `DocViewer` | Documentation, README files |
| `media-gallery` | `MediaGallery` | Images, screenshots, videos |

---

## Implementation Phases

### Phase 1: Foundation (Current)
- [x] Define tool response schema with UI hints
- [x] Create ResultRenderer component
- [x] Implement base views (Preview, JSONTree, DataTable)
- [x] Create result history store
- [x] Update IntentBar to show results
- [ ] Update MCP tools to return structured responses

### Phase 2: Core Views
- [ ] LogViewer - terminal/log queries
- [ ] FileGrid - file search results
- [ ] CodeDiffViewer - git queries
- [ ] DocViewer - documentation

### Phase 3: Advanced Views
- [ ] Timeline - event sequences
- [ ] EmailReader - Gmail integration
- [ ] MediaGallery - images/videos
- [ ] ContextSwitcher - workspace management

### Phase 4: Intelligence
- [ ] Smart view selection (LLM suggests best view)
- [ ] Result pinning/bookmarking
- [ ] Cross-result linking
- [ ] Action suggestions from LLM

---

## File Structure

```
src/
├── components/
│   ├── IntentBar.tsx          # Enhanced with result history
│   ├── results/
│   │   ├── ResultRenderer.tsx # Main dispatcher
│   │   ├── PreviewCard.tsx    # Default view
│   │   ├── JSONTree.tsx       # JSON data
│   │   ├── DataTable.tsx      # Tabular data
│   │   ├── CodeDiffViewer.tsx # (Phase 2)
│   │   ├── FileGrid.tsx       # (Phase 2)
│   │   ├── LogViewer.tsx      # (Phase 2)
│   │   └── ...
│   └── ...
├── lib/
│   ├── mcp/
│   │   ├── types.ts           # ToolResponse with UI hints
│   │   └── client.ts          # Tool execution
│   ├── intent/
│   │   ├── processor.ts       # Intent parsing
│   │   └── executor.ts        # Plan execution
│   └── stores/
│       ├── results.ts         # Result history
│       └── ...
└── ...
```

---

## Example Queries & Expected Views

| User Query | Expected View Type | Data Source |
|------------|-------------------|-------------|
| "Show recent errors" | `log-viewer` | Cloudflare Logs |
| "What files changed?" | `code-diff` | GitHub |
| "Find config files" | `file-grid` | GitHub/File System |
| "Show deployment history" | `timeline` | Vercel |
| "Emails from John" | `email-reader` | Gmail |
| "API response for /users" | `json-tree` | Custom API |
| "List all servers" | `table` | Hetzner |
| "Show test screenshots" | `media-gallery` | GitHub Artifacts |

---

## MCP Tool Response Format

```typescript
interface ToolResponse {
  // Standard fields
  success: boolean;
  data: any;
  error?: string;
  
  // UI rendering hints (NEW)
  ui?: {
    viewType: ViewType;
    layout?: 'full' | 'panel' | 'inline' | 'modal';
    title?: string;
    description?: string;
    actions?: ToolAction[];
    metadata?: {
      itemCount?: number;
      duration?: string;
      source?: string;
      [key: string]: any;
    };
  };
}

interface ToolAction {
  label: string;
  intent: string;  // Natural language for next action
  icon?: string;
  variant?: 'primary' | 'secondary' | 'danger';
}

type ViewType = 
  | 'preview'
  | 'json-tree'
  | 'table'
  | 'code-diff'
  | 'file-grid'
  | 'log-viewer'
  | 'timeline'
  | 'email-reader'
  | 'doc-viewer'
  | 'media-gallery';
```

---

## Result History Store

```typescript
interface ResultStore {
  // Current session results
  results: ResultItem[];
  
  // Pinned/important results
  pinned: string[];  // result IDs
  
  // Context groups (like workspaces)
  contexts: ContextGroup[];
  
  // Actions
  addResult: (result: ResultItem) => void;
  removeResult: (id: string) => void;
  pinResult: (id: string) => void;
  clearResults: () => void;
  createContext: (name: string) => void;
  switchContext: (id: string) => void;
}

interface ResultItem {
  id: string;
  query: string;
  timestamp: Date;
  response: ToolResponse;
  viewState?: Record<string, any>;  // Scroll position, expanded nodes, etc.
}

interface ContextGroup {
  id: string;
  name: string;
  resultIds: string[];
  createdAt: Date;
}
```

---

## Future Enhancements

### Smart View Selection
LLM suggests best view type based on query intent:
```typescript
{
  ui: {
    viewType: "timeline",  // LLM chose this
    reason: "User asked about sequence of events"
  }
}
```

### Cross-Result Linking
Results reference each other:
```typescript
{
  relatedResults: ["result-123", "result-456"],
  links: [
    { label: "Related commit", resultId: "result-123" }
  ]
}
```

### Action Suggestions
Each result suggests next actions:
```typescript
{
  ui: {
    actions: [
      { label: "Fix this error", intent: "fix the null pointer in auth handler" },
      { label: "Show related logs", intent: "show logs around 14:32" }
    ]
  }
}
```

---

## Migration Notes

### Existing Windows
- Windows remain for backward compatibility
- New queries default to result panels
- Users can still `openWindow('terminal')` explicitly

### Dock Integration
- Dock transitions from "app launcher" to "context switcher"
- Each dock item represents a workspace context
- Clicking restores all related result panels

### Backward Compatibility
- MCP tools without UI hints render as `preview`
- Existing tool calls continue to work
- Gradual migration path for tools
