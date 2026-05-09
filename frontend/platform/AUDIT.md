# EdgeRun Dashboard Audit Report

Generated: 2026-05-02
Scope: `crates/edgerun-dash-webapp`

## Classification

### Real Platform Components
| Component | Location | Notes |
|-----------|----------|-------|
| `platform/registries/app-registry.ts` | platform/registries/app-registry.ts | Correct platform service |
| `platform/registries/capability-registry.ts` | platform/registries/capability-registry.ts | Correct platform service |
| `platform/registries/tool-registry.ts` | platform/registries/tool-registry.ts | Exists but weak; needs upgrade |
| `platform/registries/component-registry.tsx` | platform/registries/component-registry.tsx | Correct platform service |
| `platform/state/node-store.ts` | platform/state/node-store.ts | Correct platform store |
| `platform/state/app-store.ts` | platform/state/app-store.ts | Correct platform store |
| `platform/state/capability-store.ts` | platform/state/capability-store.ts | Correct platform store |
| `platform/ui/useApps.ts` | platform/ui/useApps.ts | Correct hook |
| `platform/ui/useCapabilities.ts` | platform/ui/useCapabilities.ts | Correct hook |
| `platform/ui/useRuntime.ts` | platform/ui/useRuntime.ts | Correct hook |
| `platform/auth/approval-tracker.ts` | platform/auth/approval-tracker.ts | Correct tracker |
| `platform/runtime/capability-resolver.ts` | platform/runtime/capability-resolver.ts | Correct resolver |

### Old Demo Components (must be isolated or removed)
| Component | Location | Issues |
|-----------|----------|-------|
| `AIAssistant` | components/os/ai-assistant.tsx | Giant component with protocol logic, direct FS/workflow/API access, not a thin wrapper |
| `AppStore` | components/os/app-store.tsx | Exports `availableApps` hardcoded list, uses old `CAPABILITY_REGISTRY`, shadowed `useApps`, calls undefined `cacheWasm`/`checkCapabilities` |
| `ChatApp` | components/os/chat-app.tsx | Fake E2E encrypted messaging with random replies, fake user data |
| `WalletApp` | components/os/wallet-app.tsx | Fake balances (`balance: 134.57`), fake transactions, fake exchange rates |
| `WorkflowBuilder` | components/os/workflow-builder.tsx | Uses old workflow-store, directly executes workflows via `executeWorkflow`, not platform-connected |
| `ResourceMonitor` | components/os/resource-monitor.tsx | Uses web worker with fake/simulated data, not connected to real node stats |

### Mixed Components (partially platform, partially old)
| Component | Location | Issues |
|-----------|----------|-------|
| `Desktop` | components/os/desktop.tsx | Imports `availableApps` directly, uses `getAppIcon` from old launcher, iterates hardcoded app IDs, uses old `GuestContext` from `@/lib/capabilities` |
| `AppLauncher` store | stores/app-launcher.tsx | Hardcodes icons, components, window sizes; imports `availableApps`; directly maps app IDs to components |
| `ai-chat-store.ts` | stores/ai-chat-store.ts | Builds system context manually from mixed sources; imports old `systemStatsStore` (fake data) alongside platform stores |
| `desktop-store.ts` | stores/desktop-store.ts | `systemStatsStore` has fake data (`nodeCount: 12`, `ramUsage: { used: 4.2, total: 8 }`); `OpenWindowDef` uses `React.ReactNode` for icon instead of icon ID |
| `codebase-context.ts` | stores/codebase-context.ts | Standalone store that should be in platform assistant module |
| `file-system-store.ts` | stores/file-system-store.ts | Browser FS store that should be a platform service or clearly demo |

### Dead/Unused or Candidate for Deletion
| Item | Location | Notes |
|------|----------|-------|
| `stores/wasm-store.ts` | stores/wasm-store.ts | Referenced by desktop.tsx for `removeWasm` — may be replaced by platform wasm-registry |

### Platform Services (need work)
| Service | Location | Issues |
|---------|----------|-------|
| `platform/registries/tool-registry.ts` | platform/registries/tool-registry.ts | Needs upgrade: no risk class, no approval metadata, no capability checks |
| `app/api/chat/route.ts` | app/api/chat/route.ts | **Critical**: directly mutates workflows, opens folders, scans codebase, builds system context — should only proxy LLM calls |
| `app/api/chat/system-prompt.ts` | app/api/chat/system-prompt.ts | Should be in `platform/assistant/assistant-prompts.ts` |
| `@/lib/capabilities.ts` | lib/capabilities.ts | **Duplicate** of `platform/registries/capability-registry.ts` — must be removed |

---

## Detailed Findings Per Mixed/Demo Component

### 1. `components/os/ai-assistant.tsx` — MIXED / DEMO
**What state it owns**: Messages (via `aiChatStore`), view mode, system prompt, file system state (via `fileSystemStore`)
**What platform store/registry should own it**: `platform/assistant/` module, `platform/state/command-store.ts`, `platform/auth/approval-tracker.ts`
**What fake values it shows**: Simulated folder open replies, built-in system prompt editing
**What direct side effects it performs**: `openDirectory()`, `scanCodebase()`, `fetch("/api/chat")`, `launchApp()` for workflow builder with `require()`
**What should be extracted**:
- Message state → `platform/assistant/assistant-session.ts`
- System prompt → `platform/assistant/assistant-prompts.ts`
- File operations → tools in `platform/registries/tool-registry.ts`
- Workflow operations → tools in `platform/registries/tool-registry.ts`
- Context building → `platform/assistant/assistant-context.ts`

### 2. `components/os/app-store.tsx` — DEMO
**What state it owns**: None (reads from platform `useApps` but also exports its own `availableApps`)
**What platform store/registry should own it**: `platform/registries/app-registry.ts`, `platform/registries/builtin-app-registry.ts`
**What fake values it shows**: Hardcoded RAM (`"32MB"`), CPU (`"0.1%"`), prices (`"$5/mo"`), all presented as real
**What direct side effects it performs**: Calls undefined `cacheWasm()`, undefined `checkCapabilities()`, uses old `CAPABILITY_REGISTRY`
**What should be extracted**:
- `availableApps` → `platform/registries/builtin-app-registry.ts`
- AppDefinition interface → `platform/types/app-definition.ts`
- Capability checking → `platform/runtime/capability-resolver.ts`
- WASM caching → `platform/runtime/wasm-registry.ts`

### 3. `stores/app-launcher.tsx` — MIXED
**What state it owns**: Icon mapping (`getAppIcon`), component mapping (`buildAppComponent`), window size mapping
**What platform store/registry should own it**: `platform/registries/component-registry.tsx`, `platform/registries/window-registry.ts`, `platform/registries/app-registry.ts`
**What fake values it shows**: Window sizes hardcoded per app ID, RAM usage fake update (`parseFloat(app.ram) / 1000`)
**What direct side effects it performs**: `openWindow()`, `systemStatsStore.set()` to fake-update RAM
**What should be extracted**:
- `getAppIcon` → `platform/registries/component-registry.tsx` (icon lookup by ID)
- `buildAppComponent` → `platform/registries/component-registry.tsx`
- Window sizes → `platform/registries/window-registry.ts`
- Launch logic → `platform/ui/useRuntime.ts` or `platform/runtime/app-runtime.ts`

### 4. `components/os/chat-app.tsx` — DEMO (fake data)
**What state it owns**: Channels, messages, DM list — all hardcoded in `INITIAL_CHANNELS`
**What platform store/registry should own it**: Real messaging would use `platform/state/node-store.ts` + protocol; for now, should be labeled demo
**What fake values it shows**: Fake user names (Elias Voss, Priya Mehta, etc.), fake messages, random replies with `setTimeout`, claims "E2E encrypted" with lock icon
**What direct side effects it performs**: `setTimeout` to simulate replies
**What should be extracted**: Rename to `DemoChatApp`, add demo badge, remove from real app catalog

### 5. `components/os/wallet-app.tsx` — DEMO (fake data)
**What state it owns**: Balance (`134.57`), transactions (`DEMO_TXS`), send form state
**What platform store/registry should own it**: Real wallet would use protocol + `platform/state/object-store.ts`; for now, should be labeled demo
**What fake values it shows**: Fake balance, fake EDGE address (`edge1qxy2kgdygjrsqtzq2n0yrf249.run`), fake USD conversion (`${(balance * 1.84).toFixed(2)}`), fake transaction history
**What direct side effects it performs**: `setTimeout` to simulate send, fake balance update
**What should be extracted**: Label as demo wallet, remove from real catalog, or add demo badge

### 6. `app/api/chat/route.ts` — MIXED (critical)
**What state it owns**: None (stateless API route)
**What platform store/registry should own it**: Should ONLY proxy LLM. Workflow/file/code actions → tools.
**What fake values it shows**: None directly, but passes fake system data from `systemStatsStore`
**What direct side effects it performs**: `scanCodebase()`, `openDirectory()`, `createWorkflowFromAI()`, `updateWorkflowFromAI()`, `executeWorkflow()` — ALL should be tools, not API route actions
**What should be extracted**:
- Workflow actions → `platform/registries/tool-registry.ts` as tools
- File/codebase actions → `platform/registries/tool-registry.ts` as tools
- System prompt → `platform/assistant/assistant-prompts.ts`
- System data → built from platform stores only

### 7. `stores/workflow-store.ts` — OLD DEMO STORE
**What state it owns**: Workflows, executions, all in localStorage
**What platform store/registry should own it**: Platform workflow support would use `platform/state/command-store.ts` + protocol; for now, mark as demo
**What fake values it shows**: Uses `eval()` for condition action, fake execution with `setTimeout`
**What direct side effects it performs**: `fetch()` in `executeAction` for HTTP type, `eval()` for condition
**What should be extracted**: Mark as demo-only store, or move workflow logic to platform tools

### 8. `lib/capabilities.ts` — DUPLICATE
**What state it owns**: `CAPABILITY_REGISTRY`, `Capability` enum, `checkCapabilities()` function
**What platform store/registry should own it**: `platform/registries/capability-registry.ts`, `platform/runtime/capability-resolver.ts`
**What fake values it shows**: N/A
**What direct side effects it performs**: None
**What should be extracted**: DELETE. Replace all imports with platform equivalents.

---

## Summary of Violations

1. **Duplicate protocol models**: `lib/capabilities.ts` duplicates `platform/registries/capability-registry.ts`
2. **Components contain protocol/business logic**: `ai-assistant.tsx` has folder opening, codebase scanning, workflow management
3. **Fake data shown as real**: `systemStatsStore` (fake RAM/CPU/nodes), `AppDefinition` (fake RAM/CPU/price), `WalletApp` (fake balance/transactions), `ChatApp` (fake E2E encrypted messaging)
4. **No mode boundary**: No real/demo/offline mode switch
5. **API route mutates state**: `/api/chat/route.ts` creates/updates/executes workflows, opens folders, scans codebase
6. **Old stores mixed with platform**: `ai-chat-store.ts` mixes `systemStatsStore` (fake) with platform stores
7. **Hardcoded app catalog**: `availableApps` in `app-store.tsx`, used by `app-launcher.tsx` and `desktop.tsx`
8. **Tool registry is weak**: No risk class, no approval flow, no capability gating
9. **No demo labels**: Fake data presented without "Demo" badge
10. **State-changing actions bypass approvals**: `launchApp()` fakes RAM usage, workflow execution is direct
