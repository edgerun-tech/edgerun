import { atom, computed } from "nanostores"
import { persistentAtom } from "@nanostores/persistent"
import type { AppDefinition } from "@/platform/types/app-definition"

export interface LogEntry {
  id: string
  timestamp: Date
  type: "info" | "success" | "warning" | "error" | "system"
  message: string
}

export type AppSurfaceKind = "overlay" | "pinned-widget"
export type AppSurfaceSlot = "left-top" | "left-bottom" | "right-top" | "right-bottom"

export interface AppSurfaceDef {
  id: string
  appId: string
  title: string
  icon: React.ReactNode
  component: React.ReactNode
  kind: AppSurfaceKind
  dismissOnOutsideClick: boolean
  preferredSlot?: AppSurfaceSlot
  defaultSize?: { width: number; height: number }
}

/** @deprecated Use AppSurfaceDef. Kept as a transition alias for older launcher call sites. */
export type OpenWindowDef = AppSurfaceDef

export const appSurfacesStore = atom<AppSurfaceDef[]>([])
export const appSurfaceOrderStore = atom<string[]>([])
export const focusedAppSurfaceStore = atom<string | null>(null)

/** @deprecated Use appSurfacesStore. */
export const windowsStore = appSurfacesStore
/** @deprecated Use appSurfaceOrderStore. */
export const windowOrderStore = appSurfaceOrderStore
/** @deprecated Use focusedAppSurfaceStore. */
export const focusedWindowStore = focusedAppSurfaceStore

export const widgetVisibleStore = persistentAtom("edgerun:widgetVisible", false, {
  encode: String,
  decode: (v) => v === "true",
})

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

export const runningAppsStore = computed(appSurfacesStore, (surfaces) =>
  surfaces.map((surface) => surface.appId)
)

export function addLog(type: LogEntry["type"], message: string) {
  terminalLogsStore.set([...terminalLogsStore.get(), {
    id: `log-${Date.now()}-${Math.random()}`,
    timestamp: new Date(),
    type,
    message,
  }])
}

export function openAppSurface(surface: AppSurfaceDef) {
  const existing = appSurfacesStore.get().filter((candidate) => candidate.id !== surface.id)
  appSurfacesStore.set([...existing, surface])
  appSurfaceOrderStore.set([...appSurfaceOrderStore.get().filter((id) => id !== surface.id), surface.id])
  focusedAppSurfaceStore.set(surface.id)
}

/** @deprecated Use openAppSurface. */
export const openWindow = openAppSurface

export function closeAppSurface(surfaceId: string) {
  const surfaces = appSurfacesStore.get()
  const closed = surfaces.find((surface) => surface.id === surfaceId)
  appSurfacesStore.set(surfaces.filter((surface) => surface.id !== surfaceId))
  appSurfaceOrderStore.set(appSurfaceOrderStore.get().filter((id) => id !== surfaceId))

  if (focusedAppSurfaceStore.get() === surfaceId) {
    const remainingOrder = appSurfaceOrderStore.get().filter((id) => id !== surfaceId)
    focusedAppSurfaceStore.set(remainingOrder[remainingOrder.length - 1] || null)
  }

  return closed
}

/** @deprecated Use closeAppSurface. */
export const closeWindow = closeAppSurface

export function focusAppSurface(surfaceId: string) {
  focusedAppSurfaceStore.set(surfaceId)
  appSurfaceOrderStore.set(
    [...appSurfaceOrderStore.get().filter((id) => id !== surfaceId), surfaceId]
  )
}

/** @deprecated Use focusAppSurface. */
export const focusWindow = focusAppSurface
