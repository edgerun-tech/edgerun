import { atom, computed } from "nanostores"
import { fetchWasmModule, WasmSource, type WasmFetchOptions } from "@/lib/wasm/wasm-fetcher"

export interface AppPackage {
  name: string
  entry: string
  wasmObjectId: string
  wasmUrl?: string
  routes: Record<string, string>
  requiredCapabilities: string[]
  isPublic: boolean
}

export interface CachedWasm {
  name: string
  wasmUrl: string
  wasmBytes: Uint8Array
  installedAt: Date
  isPublic: boolean
  hash?: string
}

export const wasmCacheStore = atom<Map<string, CachedWasm>>(new Map())

export const installedWasmListStore = computed(wasmCacheStore, (cache) =>
  Array.from(cache.values())
)

export const installedWasmNamesStore = computed(wasmCacheStore, (cache) =>
  Array.from(cache.keys())
)

export function getWasmFromCache(name: string): CachedWasm | undefined {
  return wasmCacheStore.get().get(name)
}

export function installWasm(wasm: CachedWasm) {
  const cache = new Map(wasmCacheStore.get())
  cache.set(wasm.name, wasm)
  wasmCacheStore.set(cache)
}

export function removeWasm(name: string) {
  const cache = new Map(wasmCacheStore.get())
  const entry = cache.get(name)
  if (entry?.wasmUrl.startsWith("blob:")) {
    URL.revokeObjectURL(entry.wasmUrl)
  }
  if (cache.delete(name)) {
    wasmCacheStore.set(cache)
  }
}

export async function fetchWasmPackage(pkg: AppPackage): Promise<CachedWasm> {
  const existing = getWasmFromCache(pkg.name)
  if (existing) return existing

  const opts: WasmFetchOptions = pkg.isPublic
    ? { source: WasmSource.PublicUrl, url: pkg.wasmUrl, isPublic: true }
    : { source: WasmSource.NodeFetch, objectId: pkg.wasmObjectId, nodeId: undefined, isPublic: false }

  const result = await fetchWasmModule(opts)
  const blobUrl = URL.createObjectURL(new Blob([result.bytes], { type: "application/wasm" }))

  const wasmModule: CachedWasm = {
    name: result.name,
    wasmUrl: blobUrl,
    wasmBytes: result.bytes,
    installedAt: new Date(),
    isPublic: pkg.isPublic,
    hash: result.hash,
  }

  installWasm(wasmModule)
  return wasmModule
}

export async function fetchWasmFromUrl(name: string, url: string, isPublic = true): Promise<CachedWasm> {
  const existing = getWasmFromCache(name)
  if (existing) return existing

  const result = await fetchWasmModule({
    source: WasmSource.PublicUrl,
    url,
    isPublic,
  })

  const blobUrl = URL.createObjectURL(new Blob([result.bytes], { type: "application/wasm" }))
  const wasmModule: CachedWasm = {
    name: result.name,
    wasmUrl: blobUrl,
    wasmBytes: result.bytes,
    installedAt: new Date(),
    isPublic,
  }

  installWasm(wasmModule)
  return wasmModule
}
