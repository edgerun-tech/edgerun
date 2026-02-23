# End-to-End Flow Documentation

## Complete Data Flow: User Query → Morphable Result

This document traces the complete journey of a user query through the CloudOS AI-centric architecture.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           USER INTERFACE                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  IntentBar                                                       │    │
│  │  ┌─────────────────────────────────────────────────────────┐    │    │
│  │  │ "Show me the deployment history for my-app"              │    │    │
│  │  └─────────────────────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        INTENT PROCESSING                                 │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐  │
│  │ IntentProcessor  │ →  │ LLM Router       │ →  │ MCP Manager      │  │
│  │                  │    │ (OpenAI/Ollama)  │    │ (Tool Registry)  │  │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         EXECUTION                                        │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐  │
│  │ IntentExecutor   │ →  │ MCP Tools        │ →  │ ToolResponse[]   │  │
│  │                  │    │ (Web Workers)    │    │ (with UI hints)  │  │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         RENDERING                                        │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐  │
│  │ ResultRenderer   │ →  │ View Components  │ →  │ Result History   │  │
│  │ (Auto-detect)    │    │ (10 view types)  │    │ (LocalStorage)   │  │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Step-by-Step Flow

### Step 1: User Input (IntentBar)

**File:** `src/components/IntentBar.tsx`

```typescript
// User types query
const query = "Show me the deployment history for my-app";

// Query is sent to IntentProcessor
const plan = await intentProcessor.process(query, appContext);
```

**State Created:**
```typescript
{
  query: "Show me the deployment history for my-app",
  timestamp: Date,
  context: {
    currentRepo: "my-app",
    currentBranch: "main",
    activeIntegrations: ["github", "vercel"],
    // ...
  }
}
```

---

### Step 2: Intent Processing

**File:** `src/lib/intent/processor.ts`

#### 2.1 Get Available Tools

```typescript
const mcpTools = mcpManager.getToolsForLLM();
// Returns: [{ name: 'get_deployments', description: '...', parameters: {...} }, ...]
```

#### 2.2 Build System Prompt

```typescript
const systemPrompt = `You are the Intent Processor for browser-os...

Available Tools:
- get_deployments: Get deployment history from Vercel
- get_commits: Get commit history from GitHub
- get_logs: Get logs from Cloudflare
...

Response Format:
{
  "intent": "brief description",
  "tools": ["tool1", "tool2"],
  "steps": [...],
  "risk": "low|medium|high",
  "requiresAuth": true|false,
  "predictedResult": "what will happen"
}`;
```

#### 2.3 Call LLM

```typescript
const response = await llmRouter.route({
  messages: [
    { role: 'system', content: systemPrompt },
    { role: 'user', content: query }
  ],
  tools: llmTools,
  tool_choice: 'auto',
  temperature: 0.2
});
```

#### 2.4 Parse Response → ExecutionPlan

```typescript
// LLM returns tool calls
{
  tool_calls: [
    {
      function: {
        name: 'get_deployments',
        arguments: '{"project": "my-app", "limit": 10}'
      }
    }
  ]
}

// Converted to ExecutionPlan
{
  id: 'plan-1708123456789',
  intent: {
    raw: 'Show me the deployment history for my-app',
    verb: 'get',
    target: 'deployments',
    modifiers: ['my-app'],
    context: appContext,
    confidence: 0.9
  },
  steps: [
    {
      tool: 'get_deployments',
      args: { project: 'my-app', limit: 10 },
      description: 'Get deployment history from Vercel'
    }
  ],
  risk: 'low',
  preview: [
    { label: 'Action', value: 'Get deployment history', type: 'info' }
  ],
  requiresAuth: false,
  predictedResult: 'Will show recent deployments for my-app'
}
```

---

### Step 3: Intent Execution

**File:** `src/lib/intent/executor.ts`

#### 3.1 Execute Each Step

```typescript
for (const step of plan.steps) {
  const result = await executeStep(step, plan);
}
```

#### 3.2 Build Structured ToolResponse

```typescript
private buildToolResponse(tool: string, data: any, plan: ExecutionPlan): ToolResponse {
  // Map tool to view type
  const viewType = toolViewTypes[tool] || 'preview';
  
  // Build response based on tool
  switch (tool) {
    case 'get_deployments':
      return {
        success: true,
        data: [...], // Raw deployment data
        ui: {
          viewType: 'timeline',  // ← Key: tells UI how to render
          title: 'Deployment History',
          description: plan.predictedResult,
          metadata: {
            source: 'vercel',
            itemCount: 10,
            timestamp: new Date().toISOString()
          }
        }
      };
  }
}
```

#### 3.3 Return ExecutionResult

```typescript
{
  success: true,
  message: 'Will show recent deployments for my-app',
  steps: [
    {
      tool: 'get_deployments',
      success: true,
      result: [...],
      response: {  // ← Structured ToolResponse
        success: true,
        data: [...],
        ui: { viewType: 'timeline', ... }
      }
    }
  ],
  responses: [  // ← All ToolResponses for rendering
    { success: true, data: [...], ui: {...} }
  ]
}
```

---

### Step 4: Save to Result History

**File:** `src/components/IntentBar.tsx`

```typescript
if (result.success && result.responses) {
  // Save each tool response
  for (const response of result.responses) {
    addResult({
      query: query(),
      response,  // ← Full ToolResponse with UI hints
    });
  }
}
```

**Stored in LocalStorage:**
```typescript
{
  results: [
    {
      id: 'result-1708123456789-abc123',
      query: 'Show me the deployment history for my-app',
      timestamp: '2024-02-17T13:30:00.000Z',
      response: {
        success: true,
        data: [...],
        ui: { viewType: 'timeline', ... }
      },
      isPinned: false
    }
  ],
  contexts: [...]
}
```

---

### Step 5: Render Results

**File:** `src/components/results/ResultRenderer.tsx`

#### 5.1 Auto-Detect View Type

```typescript
const viewType = () => {
  // First check explicit UI hints
  if (props.response.ui?.viewType) {
    return props.response.ui.viewType;  // 'timeline'
  }
  
  // Fallback: auto-detect from data structure
  // ...detection logic
};
```

#### 5.2 Dispatch to View Component

```typescript
const viewComponents: Record<ViewType, any> = {
  'timeline': Timeline,
  'email-reader': EmailReader,
  'code-diff': CodeDiffViewer,
  // ... 10 view types
};

const ViewComponent = viewComponents[viewType()];
return <ViewComponent response={props.response} />;
```

---

### Step 6: Display in UI

**File:** `src/components/IntentBar.tsx`

```tsx
<Show when={showHistory()}>
  <div class="px-4 pb-4 space-y-3 max-h-[400px] overflow-y-auto">
    <For each={results()}>
      {(result) => (
        <ResultRenderer
          response={result.response}
          onAction={(intent) => {
            setQuery(intent);  // ← Click action to continue conversation
            inputRef?.focus();
          }}
        />
      )}
    </For>
  </div>
</Show>
```

**Final Rendered Output:**
```
┌─────────────────────────────────────────────────────────────┐
│ Timeline View                                               │
│ ─────────────────────────────────────────────────────────── │
│ 🚀 Deployment History                          10 events    │
│ ─────────────────────────────────────────────────────────── │
│                                                             │
│   🚀  Production Deploy - v2.3.1                    2h ago  │
│      Commit: "Fix authentication bug"                       │
│      Status: success                                        │
│                                                             │
│   🚀  Production Deploy - v2.3.0                    1d ago  │
│      Commit: "Add new dashboard"                            │
│      Status: success                                        │
│                                                             │
│   🔨  Build Failed                                  2d ago  │
│      Commit: "WIP: broken change"                           │
│      Status: error                                          │
│                                                             │
│ [Actions: View Logs] [Show Diff] [Rollback]                 │
└─────────────────────────────────────────────────────────────┘
```

---

## Complete Example: "Show me recent errors"

### 1. User Query
```
"Show me recent errors from the API"
```

### 2. ExecutionPlan Created
```typescript
{
  intent: {
    raw: "Show me recent errors from the API",
    verb: "get",
    target: "logs",
    modifiers: ["recent", "errors", "API"]
  },
  steps: [
    {
      tool: "get_logs",
      args: { 
        level: "error",
        source: "api",
        limit: 50
      },
      description: "Get error logs from API"
    }
  ]
}
```

### 3. ToolResponse Returned
```typescript
{
  success: true,
  data: [
    { level: 'error', timestamp: '2024-02-17T13:00:00Z', message: 'Connection timeout' },
    { level: 'error', timestamp: '2024-02-17T12:30:00Z', message: 'Null pointer' },
    // ... 48 more
  ],
  ui: {
    viewType: 'log-viewer',
    title: 'Error Logs',
    description: 'Recent errors from API',
    metadata: {
      source: 'api',
      itemCount: 50,
      timestamp: '2024-02-17T13:30:00Z'
    }
  }
}
```

### 4. ResultRenderer Dispatches
```typescript
viewType = 'log-viewer'  // From UI hints
→ Renders: <LogViewer response={toolResponse} />
```

### 5. User Sees
```
┌─────────────────────────────────────────────────────────────┐
│ Log Viewer                               50 lines  [Search] │
│ ─────────────────────────────────────────────────────────── │
│ [INFO] [WARN] [ERROR] [DEBUG]                              │
│ ─────────────────────────────────────────────────────────── │
│                                                             │
│ [13:00:01] ERROR  Connection timeout to database            │
│ [12:30:15] ERROR  Null pointer in auth handler              │
│ [12:15:00] ERROR  Invalid request body                      │
│ ...                                                         │
│                                                             │
│ [Auto-scroll: ✓]  [Download] [Clear]                        │
└─────────────────────────────────────────────────────────────┘
```

---

## View Type Decision Tree

```
ToolResponse.ui.viewType
         │
         ├── Explicit (from MCP tool)
         │    └── Use specified view type
         │
         └── Auto-Detect (from data structure)
              │
              ├── Has from/to/subject? → email-reader
              ├── Has url/thumbnail + image/video? → media-gallery
              ├── Has timestamp + title? → timeline
              ├── Contains markdown syntax? → doc-viewer
              ├── Contains diff format? → code-diff
              ├── Has level/timestamp/message? → log-viewer
              ├── Has path/type:file? → file-grid
              ├── Array of objects? → table
              ├── Nested object? → json-tree
              └── Default → preview
```

---

## Key Files & Responsibilities

| File | Responsibility |
|------|---------------|
| `IntentBar.tsx` | User input, result history, rendering |
| `processor.ts` | Parse query → ExecutionPlan |
| `executor.ts` | Execute plan → ToolResponse[] |
| `ResultRenderer.tsx` | Auto-detect view type, dispatch |
| `results/*.tsx` | 10 view components |
| `stores/results.ts` | Persist results to localStorage |
| `mcp/types.ts` | ToolResponse, UIHints types |

---

## Data Structures

### ToolResponse
```typescript
interface ToolResponse {
  success: boolean;
  data: any;
  error?: string;
  ui?: {
    viewType: ViewType;  // 'timeline' | 'log-viewer' | etc.
    layout?: 'full' | 'panel' | 'inline' | 'modal';
    title?: string;
    description?: string;
    actions?: Array<{
      label: string;
      intent: string;  // Natural language for next action
      variant?: 'primary' | 'secondary' | 'danger';
    }>;
    metadata?: {
      itemCount?: number;
      duration?: string;
      source?: string;
      timestamp?: string;
    };
  };
}
```

### ExecutionPlan
```typescript
interface ExecutionPlan {
  id: string;
  intent: ParsedIntent;
  steps: ExecutionStep[];
  risk: 'low' | 'medium' | 'high' | 'critical';
  preview: PreviewItem[];
  requiresAuth: boolean;
  predictedResult: string;
}
```

### ResultItem (Stored)
```typescript
interface ResultItem {
  id: string;
  query: string;
  timestamp: Date;
  response: ToolResponse;
  viewState?: Record<string, any>;
  isPinned?: boolean;
}
```

---

## Action Chaining Example

```
User: "Show deployments"
  ↓
[Timeline rendered]
  ↓
User clicks: "View Logs" action
  ↓
Query input filled: "show logs for deployment v2.3.1"
  ↓
User presses Enter
  ↓
[LogViewer rendered]
  ↓
User clicks: "Fix this error" action
  ↓
Query input filled: "fix the null pointer in auth handler"
  ↓
[CodeDiffViewer rendered with fix]
```

Each result's `ui.actions` contains natural language intents that chain into the next query!

---

## Performance Considerations

- **Result Limit:** 100 results max (auto-cleanup)
- **Context Limit:** 50 results per workspace
- **Storage:** LocalStorage (persistent across sessions)
- **Lazy Loading:** Results render on-demand
- **Virtualization:** Needed for large result sets (future)

---

## Error Handling

```typescript
try {
  const plan = await intentProcessor.process(query, context);
  if (!plan) throw new Error('Could not parse intent');
  
  const result = await intentExecutor.execute(plan);
  if (!result.success) {
    // Show error as ToolResponse
    addResult({
      query,
      response: {
        success: false,
        error: result.message,
        ui: { viewType: 'preview', title: 'Error' }
      }
    });
  }
} catch (e) {
  // Handle gracefully
}
```

---

This end-to-end flow enables natural language interaction with cloud infrastructure, with results automatically rendered in the most appropriate view for each data type.
