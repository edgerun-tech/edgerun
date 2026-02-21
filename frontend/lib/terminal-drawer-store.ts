import { createIndexedDbAdapter } from './state/indexeddb-adapter'
import terminalDevicesConfig from '../config/terminal-devices.json'

const TERM_MIN_RATIO = 0.2
const TERM_MAX_RATIO = 0.85
const TERM_DEFAULT_RATIO = 0.35

export type TerminalSplitMode = 'none' | 'split-cols' | 'split-rows'
export type DeviceStatus = 'unknown' | 'online' | 'offline'

export type TerminalPane = {
  id: string
  baseUrl: string
}

export type TerminalTab = {
  id: string
  title: string
  split: TerminalSplitMode
  panes: TerminalPane[]
}

export type TerminalDevice = {
  id: string
  name: string
  baseUrl: string
  status: DeviceStatus
  lastSeenAt: number | null
  lastConnectedAt: number | null
}

export type TerminalDrawerState = {
  open: boolean
  heightRatio: number
  autoImportTailscale: boolean
  tabs: TerminalTab[]
  activeTabId: string
  devices: TerminalDevice[]
}

type Listener = (state: TerminalDrawerState) => void
type Updater = (prev: TerminalDrawerState) => TerminalDrawerState

const persistence = createIndexedDbAdapter<TerminalDrawerState>({
  dbName: 'edgerun-frontend-ui',
  storeName: 'state',
  key: 'edgerun.frontend.termDrawer.v2'
})

let state = normalizeState(null)
let initialized = false
let persistTimer: number | null = null
const listeners = new Set<Listener>()
let seededDefaults = false

function termId(): string {
  if (typeof window !== 'undefined' && window.crypto?.randomUUID) {
    return window.crypto.randomUUID()
  }
  return `id-${Date.now()}-${Math.floor(Math.random() * 1e6)}`
}

function nowMs(): number {
  return Date.now()
}

function cloneState<T>(value: T): T {
  if (typeof globalThis.structuredClone === 'function') {
    return globalThis.structuredClone(value)
  }
  return JSON.parse(JSON.stringify(value)) as T
}

function defaultTab(index: number): TerminalTab {
  return {
    id: termId(),
    title: `Terminal ${index + 1}`,
    split: 'none',
    panes: [{ id: termId(), baseUrl: '' }]
  }
}

function normalizeState(raw: Partial<TerminalDrawerState> | null): TerminalDrawerState {
  const base = raw ?? {}
  const tabs = Array.isArray(base.tabs) && base.tabs.length > 0
    ? base.tabs.map((tab, idx) => {
      const split: TerminalSplitMode = tab?.split === 'split-cols' || tab?.split === 'split-rows' ? tab.split : 'none'
      const panes = Array.isArray(tab?.panes) && tab.panes.length > 0
        ? tab.panes.slice(0, 2).map((pane) => ({
            id: pane?.id || termId(),
            baseUrl: typeof pane?.baseUrl === 'string' ? pane.baseUrl.trim() : ''
          }))
        : [{ id: termId(), baseUrl: '' }]

      return {
        id: tab?.id || termId(),
        title: typeof tab?.title === 'string' && tab.title.trim() ? tab.title : `Terminal ${idx + 1}`,
        split,
        panes
      }
    })
    : [defaultTab(0)]

  const activeTabId = tabs.some((tab) => tab.id === base.activeTabId) ? String(base.activeTabId) : tabs[0]!.id
  const heightCandidate = typeof base.heightRatio === 'number' ? base.heightRatio : Number(base.heightRatio)
  const heightRatio = Number.isFinite(heightCandidate)
    ? Math.max(TERM_MIN_RATIO, Math.min(TERM_MAX_RATIO, heightCandidate))
    : TERM_DEFAULT_RATIO

  const devices = Array.isArray(base.devices)
    ? base.devices
      .filter((item) => item && typeof item.baseUrl === 'string' && item.baseUrl.trim().length > 0)
      .map((item, idx) => {
        const status: DeviceStatus = item.status === 'online' || item.status === 'offline' ? item.status : 'unknown'
        return {
          id: typeof item.id === 'string' && item.id.trim() ? item.id : termId(),
          name: typeof item.name === 'string' && item.name.trim() ? item.name : `Device ${idx + 1}`,
          baseUrl: item.baseUrl.trim(),
          status,
          lastSeenAt: typeof item.lastSeenAt === 'number' ? item.lastSeenAt : null,
          lastConnectedAt: typeof item.lastConnectedAt === 'number' ? item.lastConnectedAt : null
        }
      })
    : []

  return {
    open: Boolean(base.open),
    heightRatio,
    autoImportTailscale: typeof base.autoImportTailscale === 'boolean' ? base.autoImportTailscale : true,
    tabs,
    activeTabId,
    devices
  }
}

function notify(): void {
  for (const listener of listeners) listener(state)
}

function queuePersist(): void {
  if (typeof window === 'undefined') return
  if (persistTimer !== null) window.clearTimeout(persistTimer)
  persistTimer = window.setTimeout(() => {
    void persistence.set(state)
    persistTimer = null
  }, 120)
}

function apply(update: Updater): void {
  state = normalizeState(update(cloneState(state)))
  queuePersist()
  notify()
}

function activeTab(current: TerminalDrawerState): TerminalTab {
  return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]!
}

function lastUsedDevice(current: TerminalDrawerState): TerminalDevice | null {
  if (current.devices.length === 0) return null
  const sorted = [...current.devices].sort((left, right) => {
    const l = left.lastConnectedAt ?? 0
    const r = right.lastConnectedAt ?? 0
    if (l !== r) return r - l
    return left.name.localeCompare(right.name)
  })
  return sorted[0] ?? null
}

export function ensureTerminalDrawerStore(): void {
  if (initialized) return
  initialized = true
  if (typeof window === 'undefined') return

  void persistence.get().then((saved) => {
    if (!saved) return
    state = normalizeState(saved)
    seedKnownDevices()
    notify()
  })
  seedKnownDevices()
}

export function getTerminalDrawerState(): TerminalDrawerState {
  return state
}

export function subscribeTerminalDrawer(listener: Listener): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export const terminalDrawerActions = {
  toggle(): void {
    apply((prev) => ({ ...prev, open: !prev.open }))
  },

  setOpen(open: boolean): void {
    apply((prev) => ({ ...prev, open }))
  },

  setAutoImportTailscale(enabled: boolean): void {
    apply((prev) => ({ ...prev, autoImportTailscale: enabled }))
  },

  addTab(): void {
    apply((prev) => {
      const tab = defaultTab(prev.tabs.length)
      return {
        ...prev,
        open: true,
        tabs: [...prev.tabs, tab],
        activeTabId: tab.id
      }
    })
  },

  closeActiveTab(): void {
    apply((prev) => {
      if (prev.tabs.length <= 1) return prev
      const idx = prev.tabs.findIndex((tab) => tab.id === prev.activeTabId)
      if (idx < 0) return prev
      const tabs = prev.tabs.filter((_, i) => i !== idx)
      const nextIdx = Math.max(0, idx - 1)
      const nextTab = tabs[nextIdx] ?? tabs[0]
      if (!nextTab) return prev
      return {
        ...prev,
        tabs,
        activeTabId: nextTab.id
      }
    })
  },

  setActiveTab(id: string): void {
    apply((prev) => {
      if (!prev.tabs.some((tab) => tab.id === id)) return prev
      return {
        ...prev,
        activeTabId: id,
        open: true
      }
    })
  },

  setSplit(mode: TerminalSplitMode): void {
    apply((prev) => {
      const tab = activeTab(prev)
      const tabs = prev.tabs.map((entry) => {
        if (entry.id !== tab.id) return entry
        if (mode === 'none') {
          return {
            ...entry,
            split: 'none' as const,
            panes: [entry.panes[0] ?? { id: termId(), baseUrl: '' }]
          }
        }
        const panes = entry.panes.length < 2
          ? [...entry.panes, { id: termId(), baseUrl: entry.panes[0]?.baseUrl ?? '' }]
          : entry.panes.slice(0, 2)
        return {
          ...entry,
          split: mode,
          panes
        }
      })
      return { ...prev, tabs, open: true }
    })
  },

  connectActiveTabToDevice(deviceId: string): void {
    apply((prev) => {
      const device = prev.devices.find((item) => item.id === deviceId)
      if (!device) return prev
      const tab = activeTab(prev)
      const tabs = prev.tabs.map((entry) => entry.id === tab.id
        ? {
            ...entry,
            panes: entry.panes.map((pane) => ({ ...pane, baseUrl: device.baseUrl }))
          }
        : entry
      )
      const devices = prev.devices.map((item) => item.id === deviceId
        ? { ...item, lastConnectedAt: nowMs() }
        : item
      )
      return {
        ...prev,
        open: true,
        tabs,
        devices
      }
    })
  },

  restoreLastDeviceOnActiveTab(): void {
    apply((prev) => {
      const tab = activeTab(prev)
      const hasConnectedPane = tab.panes.some((pane) => pane.baseUrl.trim().length > 0)
      if (hasConnectedPane) return prev

      const device = lastUsedDevice(prev)
      if (!device) return prev

      const tabs = prev.tabs.map((entry) => entry.id === tab.id
        ? {
            ...entry,
            panes: entry.panes.map((pane) => ({ ...pane, baseUrl: device.baseUrl }))
          }
        : entry
      )

      return {
        ...prev,
        tabs
      }
    })
  },

  setHeightRatio(nextRatio: number): void {
    const clamped = Math.max(TERM_MIN_RATIO, Math.min(TERM_MAX_RATIO, nextRatio))
    apply((prev) => ({ ...prev, open: true, heightRatio: clamped }))
  },

  addDevice(name: string, baseUrl: string): void {
    const trimmedUrl = baseUrl.trim()
    const trimmedName = name.trim()
    if (!trimmedUrl) return
    apply((prev) => {
      const exists = prev.devices.some((item) => item.baseUrl === trimmedUrl)
      if (exists) return prev
      return {
        ...prev,
        devices: [...prev.devices, {
          id: termId(),
          name: trimmedName || `Device ${prev.devices.length + 1}`,
          baseUrl: trimmedUrl,
          status: 'unknown',
          lastSeenAt: null,
          lastConnectedAt: null
        }]
      }
    })
  },

  removeDevice(deviceId: string): void {
    apply((prev) => ({
      ...prev,
      devices: prev.devices.filter((item) => item.id !== deviceId)
    }))
  },

  markDeviceStatus(deviceId: string, status: DeviceStatus): void {
    apply((prev) => ({
      ...prev,
      devices: prev.devices.map((item) => item.id === deviceId
        ? { ...item, status, lastSeenAt: nowMs() }
        : item
      )
    }))
  }
}

function seedKnownDevices(): void {
  if (typeof window === 'undefined' || seededDefaults) return
  seededDefaults = true

  const candidates = new Map<string, string>()
  const configDefaults = Array.isArray(terminalDevicesConfig.defaults) ? terminalDevicesConfig.defaults : []
  for (const entry of configDefaults) {
    const baseUrl = typeof entry?.baseUrl === 'string' ? entry.baseUrl.trim() : ''
    if (!baseUrl) continue
    const name = typeof entry?.name === 'string' && entry.name.trim() ? entry.name.trim() : 'Device'
    candidates.set(baseUrl, name)
  }

  candidates.set(window.location.origin, 'Current Origin')

  if (state.devices.length > 0) return
  let added = false
  for (const [baseUrl, name] of candidates.entries()) {
    if (state.devices.some((item) => item.baseUrl === baseUrl)) continue
    state.devices.push({
      id: termId(),
      name,
      baseUrl,
      status: 'unknown',
      lastSeenAt: null,
      lastConnectedAt: null
    })
    added = true
  }
  if (added) {
    notify()
    queuePersist()
  }
}

export function getTerminalPaneSrc(baseUrl: string, paneId: string): string {
  const target = (baseUrl || '').trim()
  if (!target) return ''
  try {
    const origin = typeof window !== 'undefined' ? window.location.origin : 'http://localhost'
    const url = new URL(target, origin)
    url.searchParams.set('embed', '1')
    url.searchParams.set('sid', paneId)
    return url.toString()
  } catch {
    return ''
  }
}
