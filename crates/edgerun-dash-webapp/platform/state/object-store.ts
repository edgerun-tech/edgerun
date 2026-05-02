/**
 * Single source of truth for protocol objects.
 * Tracks object metadata, content, and cache state.
 */

import { atom, computed } from "nanostores"
import type { ObjectMetadata } from "@/platform/protocol/objects"
import { protocolClient } from "@/platform/protocol/client"

export interface ObjectStoreState {
  objects: Map<string, ObjectMetadata>
  contentCache: Map<string, Uint8Array>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: ObjectStoreState = {
  objects: new Map(),
  contentCache: new Map(),
  isLoading: false,
  error: null,
  lastRefresh: null,
}

export const objectStore = atom<ObjectStoreState>(initialState)

export const allObjects = computed(objectStore, (s) =>
  Array.from(s.objects.values()),
)

export function getObjectMetadata(objectId: string): ObjectMetadata | undefined {
  return objectStore.get().objects.get(objectId)
}

export function getCachedContent(objectId: string): Uint8Array | undefined {
  return objectStore.get().contentCache.get(objectId)
}

export async function fetchObjectMetadata(
  objectId: string,
): Promise<ObjectMetadata | null> {
  const state = objectStore.get()
  objectStore.set({ ...state, isLoading: true, error: null })

  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/object/${objectId}/metadata`,
    })

    if (response.status === 200) {
      const text = new TextDecoder().decode(response.body)
      const metadata = JSON.parse(text) as ObjectMetadata
      const newObjects = new Map(objectStore.get().objects)
      newObjects.set(objectId, metadata)
      objectStore.set({
        ...objectStore.get(),
        objects: newObjects,
        isLoading: false,
        lastRefresh: new Date().toISOString(),
      })
      return metadata
    }
    throw new Error(`Failed to fetch metadata: ${response.status}`)
  } catch (err) {
    objectStore.set({
      ...objectStore.get(),
      isLoading: false,
      error: err instanceof Error ? err.message : "Unknown error",
    })
    return null
  }
}

export async function fetchObjectContent(
  objectId: string,
): Promise<Uint8Array | null> {
  const cached = getCachedContent(objectId)
  if (cached) return cached

  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/object/${objectId}`,
    })

    if (response.status === 200) {
      const newCache = new Map(objectStore.get().contentCache)
      newCache.set(objectId, response.body)
      objectStore.set({
        ...objectStore.get(),
        contentCache: newCache,
      })
      return response.body
    }
    return null
  } catch {
    return null
  }
}

export function invalidateObjectCache(objectId: string): void {
  const state = objectStore.get()
  const newCache = new Map(state.contentCache)
  newCache.delete(objectId)
  const newObjects = new Map(state.objects)
  newObjects.delete(objectId)
  objectStore.set({
    ...state,
    contentCache: newCache,
    objects: newObjects,
  })
}
