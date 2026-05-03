/**
 * Single registry for Wasm artifacts.
 * Resolves wasm_object ObjectRef, caches wasm bytes/blobURLs, validates hashes.
 */

import { atom, computed } from "nanostores"
import { edgerun } from "@/gen/edgerun/v0/common"
import { protocolClient } from "@/platform/protocol/client"

export interface WasmModule {
  name: string
  objectRef: edgerun.v0.common.ObjectRef
  wasmUrl: string
  wasmBytes: Uint8Array
  hash?: string
  installedAt: string
  isPublic: boolean
}

export interface WasmRegistryState {
  modules: Map<string, WasmModule>
  cache: Map<string, Uint8Array>
}

const initialState: WasmRegistryState = {
  modules: new Map(),
  cache: new Map(),
}

export const wasmRegistry = atom<WasmRegistryState>(initialState)

export const allWasmModules = computed(wasmRegistry, (s) =>
  Array.from(s.modules.values()),
)

export function resolveWasmObject(
  objectRef: edgerun.v0.common.ObjectRef,
): WasmModule | undefined {
  return Array.from(wasmRegistry.get().modules.values()).find(
    (m) => m.objectRef.object_id.toString() === objectRef.object_id.toString(),
  )
}

export async function loadWasmForApp(
  appId: string,
  objectRef: edgerun.v0.common.ObjectRef,
): Promise<WasmModule | null> {
  const existing = resolveWasmObject(objectRef)
  if (existing) return existing

  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/object/${Buffer.from(objectRef.object_id).toString("hex")}`,
    })
    if (response.status !== 200) throw new Error(`Failed: ${response.status}`)
    const bytes = response.body

    const blob = new Blob([bytes], { type: "application/wasm" })
    const url = URL.createObjectURL(blob)
    const hash = await crypto.subtle
      .digest("SHA-256", bytes)
      .then((h) =>
        Array.from(new Uint8Array(h))
          .map((b) => b.toString(16).padStart(2, "0"))
          .join(""),
      )

    const module: WasmModule = {
      name: appId,
      objectRef,
      wasmUrl: url,
      wasmBytes: bytes,
      hash,
      installedAt: new Date().toISOString(),
      isPublic: false,
    }

    const state = wasmRegistry.get()
    const newModules = new Map(state.modules)
    newModules.set(appId, module)
    const newCache = new Map(state.cache)
    newCache.set(Buffer.from(objectRef.object_id).toString("hex"), bytes)
    wasmRegistry.set({
      modules: newModules,
      cache: newCache,
    })

    return module
  } catch (err) {
    console.error("Failed to load WASM:", err)
    return null
  }
}

export function getWasmUrl(appId: string): string | undefined {
  return wasmRegistry.get().modules.get(appId)?.wasmUrl
}

export function getWasmBytes(objectIdHex: string): Uint8Array | undefined {
  return wasmRegistry.get().cache.get(objectIdHex)
}

export function removeWasm(appId: string): void {
  const state = wasmRegistry.get()
  const module = state.modules.get(appId)
  if (module?.wasmUrl.startsWith("blob:")) {
    URL.revokeObjectURL(module.wasmUrl)
  }
  const newModules = new Map(state.modules)
  newModules.delete(appId)
  const newCache = new Map(state.cache)
  if (module) {
    newCache.delete(Buffer.from(module.objectRef.object_id).toString("hex"))
  }
  wasmRegistry.set({
    modules: newModules,
    cache: newCache,
  })
}

export async function validateWasmHash(
  objectIdHex: string,
  expectedHash: string,
): Promise<boolean> {
  const bytes = wasmRegistry.get().cache.get(objectIdHex)
  if (!bytes) return false

  const actualHash = await crypto.subtle
    .digest("SHA-256", bytes)
    .then((h) =>
      Array.from(new Uint8Array(h))
        .map((b) => b.toString(16).padStart(2, "0"))
        .join(""),
    )

  return actualHash === expectedHash
}
