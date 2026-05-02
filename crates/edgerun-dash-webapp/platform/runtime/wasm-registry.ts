/**
 * Single registry for Wasm artifacts.
 * Resolves wasm_object ObjectRef, caches wasm bytes/blob URLs, validates hashes.
 */

import { atom, computed } from "nanostores"
import type { ObjectRef } from "@/platform/protocol/refs"
import { fetchObject } from "@/platform/protocol/objects"
import { protocolClient } from "@/platform/protocol/client"

export interface WasmModule {
  name: string
  objectRef: ObjectRef
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
  objectRef: ObjectRef,
): WasmModule | undefined {
  return Array.from(wasmRegistry.get().modules.values()).find(
    (m) => m.objectRef.objectId === objectRef.objectId,
  )
}

export async function loadWasmForApp(
  appId: string,
  objectRef: ObjectRef,
): Promise<WasmModule | null> {
  const existing = resolveWasmObject(objectRef)
  if (existing) return existing

  try {
    const bytes = await fetchObject(objectRef)
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
    newCache.set(objectRef.objectId, bytes)
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

export function getWasmBytes(objectId: string): Uint8Array | undefined {
  return wasmRegistry.get().cache.get(objectId)
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
    newCache.delete(module.objectRef.objectId)
  }
  wasmRegistry.set({
    modules: newModules,
    cache: newCache,
  })
}

export async function validateWasmHash(
  objectId: string,
  expectedHash: string,
): Promise<boolean> {
  const bytes = wasmRegistry.get().cache.get(objectId)
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
