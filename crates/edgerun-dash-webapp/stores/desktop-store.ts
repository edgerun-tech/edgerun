import { atom, computed } from "nanostores"
import { persistentAtom } from "@nanostores/persistent"
import type { AppDefinition } from "@/platform/types/app-definition"

export interface LogEntry {
  id: string
  timestamp: Date
  type: "info" | "success" | "warning" | "error" | "system"
  message: string
}

export interface OpenWindowDef {
  id: string
  appId: string
  title: string
  icon: React.ReactNode
  component: React.ReactNode
  defaultPosition: { x: number; y: number }
  defaultSize: { width: number; height: number }
}

export const stageModeStore = persistentAtom("edgerun:stageMode", false, {
  encode: String,
  decode: (v) => v === "true",
})

export const widgetVisibleStore = persistentAtom("edgerun:widgetVisible", false, {
  encode: String,
  decode: (v) => v === "true",
})

export const windowsStore = atom<OpenWindowDef[]>([])
export const windowOrderStore = atom<string[]>([])
export const focusedWindowStore = atom<string | null>(null)
export const terminalLogsStore = atom<LogEntry[]>([])

export const systemStatsStore = atom({
  nodeCount: 12,
  activeSessions: 3,
  ramUsage: { used: 4.2, total: 8 },
  isConnected: true,
})

export const hasBootedStore = atom(false)

export interface PendingGate {
  app: AppDefinition
  blocked: string[]
}
export const pendingGateStore = atom<PendingGate | null>(null)

export const runningAppsStore = computed(windowsStore, (wins) =>
  wins.map((w) => w.appId)
)

export function addLog(type: LogEntry["type"], message: string) {
  terminalLogsStore.set([...terminalLogsStore.get(), {
    id: `log-${Date.now()}-${Math.random()}`,
    timestamp: new Date(),
    type,
    message,
  }])
}

export function openWindow(win: OpenWindowDef) {
  windowsStore.set([...windowsStore.get(), win])
  windowOrderStore.set([...windowOrderStore.get(), win.id])
  focusedWindowStore.set(win.id)
}

export function closeWindow(windowId: string) {
  const wins = windowsStore.get()
  const closed = wins.find((w) => w.id === windowId)
  windowsStore.set(wins.filter((w) => w.id !== windowId))
  windowOrderStore.set(windowOrderStore.get().filter((id) => id !== windowId))
  if (focusedWindowStore.get() === windowId) {
    const order = windowOrderStore.get()
    focusedWindowStore.set(order[order.length - 1] || null)
  }
  return closed
}

export function focusWindow(windowId: string) {
  focusedWindowStore.set(windowId)
  windowOrderStore.set(
    [...windowOrderStore.get().filter((id) => id !== windowId), windowId]
  )
}
