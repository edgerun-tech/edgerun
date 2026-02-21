import { createIndexedDbAdapter } from './state/indexeddb-adapter'

const TERM_MIN_RATIO = 0.2
const TERM_MAX_RATIO = 0.85
const TERM_DEFAULT_RATIO = 0.35

export type TerminalSplitMode = 'none' | 'split-cols' | 'split-rows'

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

export type TerminalDrawerState = {
  open: boolean
  heightRatio: number
  baseUrl: string
  tabs: TerminalTab[]
  activeTabId: string
}

type Listener = (state: TerminalDrawerState) => void

type Updater = (prev: TerminalDrawerState) => TerminalDrawerState

const persistence = createIndexedDbAdapter<TerminalDrawerState>({
  dbName: 'edgerun-frontend-ui',
  storeName: 'state',
  key: 'edgerun.frontend.termDrawer.v1'
})

let state = normalizeState(null)
let initialized = false
let persistTimer: number | null = null
const listeners = new Set<Listener>()

function termId(): string {
  if (typeof window !== 'undefined' && window.crypto?.randomUUID) {
    return window.crypto.randomUUID()
  }
  return `id-${Date.now()}-${Math.floor(Math.random() * 1e6)}`
}

function cloneState<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function defaultTab(index: number, baseUrl: string): TerminalTab {
  return {
    id: termId(),
    title: `Terminal ${index + 1}`,
    split: 'none',
    panes: [{ id: termId(), baseUrl }]
  }
}

function normalizeState(raw: Partial<TerminalDrawerState> | null): TerminalDrawerState {
  const base = raw ?? {}
  const globalBase = typeof base.baseUrl === 'string' && base.baseUrl.trim() ? base.baseUrl : '/term/'

  const tabs = Array.isArray(base.tabs) && base.tabs.length > 0
    ? base.tabs.map((tab, idx) => {
      const split: TerminalSplitMode = tab?.split === 'split-cols' || tab?.split === 'split-rows' ? tab.split : 'none'
      const panes = Array.isArray(tab?.panes) && tab.panes.length > 0
        ? tab.panes.slice(0, 2).map((pane) => ({
            id: pane?.id || termId(),
            baseUrl: typeof pane?.baseUrl === 'string' && pane.baseUrl.trim() ? pane.baseUrl : globalBase
          }))
        : [{ id: termId(), baseUrl: globalBase }]

      return {
        id: tab?.id || termId(),
        title: typeof tab?.title === 'string' && tab.title.trim() ? tab.title : `Terminal ${idx + 1}`,
        split,
        panes
      }
    })
    : [defaultTab(0, globalBase)]

  const activeTabId = tabs.some((tab) => tab.id === base.activeTabId) ? String(base.activeTabId) : tabs[0]!.id
  const heightCandidate = typeof base.heightRatio === 'number' ? base.heightRatio : Number(base.heightRatio)
  const heightRatio = Number.isFinite(heightCandidate)
    ? Math.max(TERM_MIN_RATIO, Math.min(TERM_MAX_RATIO, heightCandidate))
    : TERM_DEFAULT_RATIO

  return {
    open: Boolean(base.open),
    heightRatio,
    baseUrl: globalBase,
    tabs,
    activeTabId
  }
}

function notify(): void {
  for (const listener of listeners) {
    listener(state)
  }
}

function queuePersist(): void {
  if (typeof window === 'undefined') return
  if (persistTimer !== null) window.clearTimeout(persistTimer)
  persistTimer = window.setTimeout(() => {
    void persistence.set(state)
    persistTimer = null
  }, 100)
}

function apply(update: Updater): void {
  const next = normalizeState(update(cloneState(state)))
  state = next
  queuePersist()
  notify()
}

function activeTab(current: TerminalDrawerState): TerminalTab {
  return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]!
}

export function ensureTerminalDrawerStore(): void {
  if (initialized) return
  initialized = true
  if (typeof window === 'undefined') return

  void persistence.get().then((saved) => {
    if (!saved) return
    state = normalizeState(saved)
    notify()
  })
}

export function getTerminalDrawerState(): TerminalDrawerState {
  return state
}

export function subscribeTerminalDrawer(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export const terminalDrawerActions = {
  toggle(): void {
    apply((prev) => ({ ...prev, open: !prev.open }))
  },

  setOpen(open: boolean): void {
    apply((prev) => ({ ...prev, open }))
  },

  addTab(): void {
    apply((prev) => {
      const tab = defaultTab(prev.tabs.length, prev.baseUrl)
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
            panes: [entry.panes[0] ?? { id: termId(), baseUrl: prev.baseUrl }]
          }
        }
        const panes = entry.panes.length < 2
          ? [...entry.panes, { id: termId(), baseUrl: entry.panes[0]?.baseUrl ?? prev.baseUrl }]
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

  setBaseUrl(value: string): void {
    const nextUrl = value.trim()
    if (!nextUrl) return
    apply((prev) => {
      const tab = activeTab(prev)
      const tabs = prev.tabs.map((entry) => entry.id === tab.id
        ? {
            ...entry,
            panes: entry.panes.map((pane) => ({ ...pane, baseUrl: nextUrl }))
          }
        : entry
      )
      return {
        ...prev,
        baseUrl: nextUrl,
        tabs
      }
    })
  },

  setHeightRatio(nextRatio: number): void {
    const clamped = Math.max(TERM_MIN_RATIO, Math.min(TERM_MAX_RATIO, nextRatio))
    apply((prev) => ({ ...prev, open: true, heightRatio: clamped }))
  }
}

export function getTerminalPaneSrc(baseUrl: string, paneId: string): string {
  try {
    const origin = typeof window !== 'undefined' ? window.location.origin : 'http://localhost'
    const url = new URL(baseUrl || '/term/', origin)
    url.searchParams.set('embed', '1')
    url.searchParams.set('sid', paneId)
    return url.toString()
  } catch {
    return `/term/?embed=1&sid=${encodeURIComponent(paneId)}`
  }
}
