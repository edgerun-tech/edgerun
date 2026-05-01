import { fetchWasmModule, WasmSource, type WasmFetchOptions } from "./wasm-fetcher"

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

class WasmRegistry {
  private cache = new Map<string, CachedWasm>()
  private listeners: Set<() => void> = new Set()

  install(wasm: CachedWasm) {
    this.cache.set(wasm.name, wasm)
    this.notify()
  }

  get(name: string): CachedWasm | undefined {
    return this.cache.get(name)
  }

  list(): CachedWasm[] {
    return Array.from(this.cache.values())
  }

  remove(name: string) {
    if (this.cache.has(name)) {
      const entry = this.cache.get(name)!
      if (entry.wasmUrl.startsWith("blob:")) {
        URL.revokeObjectURL(entry.wasmUrl)
      }
      this.cache.delete(name)
      this.notify()
    }
  }

  async fetchPackage(pkg: AppPackage): Promise<CachedWasm> {
    const existing = this.cache.get(pkg.name)
    if (existing) return existing

    const opts: WasmFetchOptions = pkg.isPublic
      ? { source: WasmSource.PublicUrl, url: pkg.wasmUrl, isPublic: true }
      : { source: WasmSource.NodeFetch, objectId: pkg.wasmObjectId, nodeId: undefined, isPublic: false }

    const result = await fetchWasmModule(opts)
    const blobUrl = URL.createObjectURL(new Blob([result.bytes], { type: "application/wasm" }))

    const cached: CachedWasm = {
      name: result.name,
      wasmUrl: blobUrl,
      wasmBytes: result.bytes,
      installedAt: new Date(),
      isPublic: pkg.isPublic,
      hash: result.hash,
    }
    this.cache.set(pkg.name, cached)
    this.notify()
    return cached
  }

  async fetchFromUrl(name: string, url: string, isPublic: boolean = true): Promise<CachedWasm> {
    const existing = this.cache.get(name)
    if (existing) return existing

    const result = await fetchWasmModule({
      source: WasmSource.PublicUrl,
      url,
      isPublic,
    })

    const blobUrl = URL.createObjectURL(new Blob([result.bytes], { type: "application/wasm" }))
    const cached: CachedWasm = {
      name: result.name,
      wasmUrl: blobUrl,
      wasmBytes: result.bytes,
      installedAt: new Date(),
      isPublic,
    }
    this.cache.set(name, cached)
    this.notify()
    return cached
  }

  subscribe(fn: () => void) {
    this.listeners.add(fn)
    return () => this.listeners.delete(fn)
  }

  private notify() {
    for (const fn of this.listeners) fn()
  }
}

export const wasmRegistry = new WasmRegistry()
