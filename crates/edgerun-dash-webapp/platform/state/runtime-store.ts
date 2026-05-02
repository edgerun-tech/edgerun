/**
 * Single source of truth for runtime state (WASM modules, execution state).
 */

import { atom, computed } from "nanostores"

export interface RuntimeApp {
  appId: string
  wasmUrl?: string
  wasmBytes?: Uint8Array
  isRunning: boolean
  startTime?: string
  pid?: number
}

export interface RuntimeStoreState {
  runningApps: Map<string, RuntimeApp>
  wasmCache: Map<string, { url: string; bytes: Uint8Array; hash?: string }>
  isLoading: boolean
  error: string | null
}

const initialState: RuntimeStoreState = {
  runningApps: new Map(),
  wasmCache: new Map(),
  isLoading: false,
  error: null,
}

export const runtimeStore = atom<RuntimeStoreState>(initialState)

export const runningAppsList = computed(runtimeStore, (s) =>
  Array.from(s.runningApps.values()),
)

export function startRuntimeApp(appId: string, wasmUrl?: string): void {
  const state = runtimeStore.get()
  const newRunning = new Map(state.runningApps)
  newRunning.set(appId, {
    appId,
    wasmUrl,
    isRunning: true,
    startTime: new Date().toISOString(),
  })
  runtimeStore.set({ ...state, runningApps: newRunning })
}

export function stopRuntimeApp(appId: string): void {
  const state = runtimeStore.get()
  const newRunning = new Map(state.runningApps)
  newRunning.delete(appId)
  runtimeStore.set({ ...state, runningApps: newRunning })
}

export function cacheWasm(
  name: string,
  bytes: Uint8Array,
  hash?: string,
): string {
  const state = runtimeStore.get()
  const blob = new Blob([bytes], { type: "application/wasm" })
  const url = URL.createObjectURL(blob)
  const newCache = new Map(state.wasmCache)
  newCache.set(name, { url, bytes, hash })
  runtimeStore.set({ ...state, wasmCache: newCache })
  return url
}

export function getCachedWasm(name: string): Uint8Array | undefined {
  return runtimeStore.get().wasmCache.get(name)?.bytes
}

export function getWasmUrl(name: string): string | undefined {
  return runtimeStore.get().wasmCache.get(name)?.url
}

export function removeWasmCache(name: string): void {
  const state = runtimeStore.get()
  const entry = state.wasmCache.get(name)
  if (entry?.url.startsWith("blob:")) {
    URL.revokeObjectURL(entry.url)
  }
  const newCache = new Map(state.wasmCache)
  newCache.delete(name)
  runtimeStore.set({ ...state, wasmCache: newCache })
}
