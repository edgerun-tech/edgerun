import { atom } from "nanostores"
import { systemStatsStore, windowsStore } from "./desktop-store"
import { authStore } from "./auth-store"
import { nodeStore } from "@/platform/state/node-store"
import { appStore, installedApps } from "@/platform/state/app-store"
import { capabilityStore, availableCapabilities } from "@/platform/state/capability-store"
import { getWorkflowsForAI } from "./workflow-store"

export interface ChatMessage {
  id: string
  role: "user" | "assistant"
  content: string
  timestamp: number
}

export interface ToolCall {
  name: string
  args: Record<string, unknown>
  result?: unknown
}

export interface AIChatStore {
  messages: ChatMessage[]
  isLoading: boolean
}

const STORAGE_KEY = "edgerun_ai_chat"

function loadFromStorage(): ChatMessage[] {
  if (typeof window === "undefined") return []
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return []
  try {
    const msgs = JSON.parse(raw) as ChatMessage[]
    return msgs.filter(m => 
      m.id && m.role && m.content && typeof m.timestamp === "number"
    )
  } catch {
    return []
  }
}

function saveToStorage(messages: ChatMessage[]) {
  if (typeof window === "undefined") return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(messages))
}

export const aiChatStore = atom<AIChatStore>({
  messages: [],
  isLoading: false,
})

if (typeof window !== "undefined") {
  const storedMessages = loadFromStorage()
  aiChatStore.set({
    messages: storedMessages,
    isLoading: false,
  })
}

export function addMessage(role: "user" | "assistant", content: string) {
  const store = aiChatStore.get()
  const newMessage: ChatMessage = {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    role,
    content,
    timestamp: Date.now(),
  }
  const newMessages = [...store.messages, newMessage]
  aiChatStore.set({ ...store, messages: newMessages })
  saveToStorage(newMessages)
}

export function setLoading(loading: boolean) {
  const store = aiChatStore.get()
  aiChatStore.set({ ...store, isLoading: loading })
}

export function clearChat() {
  aiChatStore.set({ messages: [], isLoading: false })
  saveToStorage([])
}

export function getSystemContext(): string {
  const chat = aiChatStore.get()
  const stats = systemStatsStore.get()
  const auth = authStore.get()
  const node = nodeStore.get()
  const apps = appStore.get()
  const caps = capabilityStore.get()
  const windows = windowsStore.get()

  const runningApps = windows.map(w => w.appId).filter(id => id !== "app-store" && id !== "app-studio")

  const appList = Array.from(apps.apps.values())
    .slice(0, 10)
    .map(a => `- ${a.name || a.appId} (${a.version || "unknown"})`)
    .join("\n")

  const capabilityList = Array.from(caps.descriptors.values())
    .slice(0, 8)
    .map(c => `- ${c.capabilityId}: ${c.description || c.capabilityType}`)
    .join("\n")

  return `
## Session Context

**User**: ${auth.username || "guest"} (${auth.authState})
**Message Count**: ${chat.messages.length}

## System Status

- **Nodes Connected**: ${stats.nodeCount}
- **Active Sessions**: ${stats.activeSessions}
- **RAM Usage**: ${stats.ramUsage.used.toFixed(1)} / ${stats.ramUsage.total} GB (${Math.round(stats.ramUsage.used / stats.ramUsage.total * 100)}%)
- **Connection Status**: ${stats.isConnected ? "Connected" : "Disconnected"}

## Local Node

- **Node ID**: ${node.currentNode?.nodeId || "Not registered"}
- **Health**: ${node.currentNode?.health || "Unknown"}
- **Runtime**: ${node.currentNode?.runtimeVersion || "Unknown"}
- **Sync Status**: ${node.currentNode?.syncStatus || "Unknown"}

## Running Applications (${runningApps.length})

${runningApps.length > 0 ? runningApps.map(id => `- ${id}`).join("\n") : "No apps running"}

## Installed Apps (${appList.length > 0 ? appList.length : 0})

${appList || "No apps installed"}

## Available Capabilities (${caps.descriptors.size})

${capabilityList || "No capabilities available"}

## Workflows

${getWorkflowsForAI()}

You can help users with:
- Deploying and managing WASM applications
- Checking node health and status
- Configuring capabilities and permissions
- Viewing system metrics
- Managing installed apps
- Creating and running automation workflows
`
}