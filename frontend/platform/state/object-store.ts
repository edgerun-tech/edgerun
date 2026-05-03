/**
 * Object store for tracking objects and their metadata.
 * Uses generated protobuf types where available.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import { edgerun } from "@/gen/edgerun/v0/common"

export interface ObjectMetadataView {
  objectId: string
  objectType: string
  sizeBytes: number
  contentHash: string
  storedAt: string
  ownerNodeId?: string
  isWasm: boolean
  isPublic: boolean
}

export interface ObjectStoreState {
  objects: Map<string, ObjectMetadataView>
  isLoading: boolean
  error: string | null
}

const initialState: ObjectStoreState = {
  objects: new Map(),
  isLoading: false,
  error: null,
}

export const objectStore = atom<ObjectStoreState>(initialState)

export const storedObjects = computed(objectStore, (s) =>
  Array.from(s.objects.values()),
)

export async function fetchObjectMetadata(
  objectId: string,
): Promise<ObjectMetadataView | null> {
  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/object/${objectId}/metadata`,
    })
    if (response.status !== 200) return null
    return JSON.parse(new TextDecoder().decode(response.body)) as ObjectMetadataView
  } catch {
    return null
  }
}
