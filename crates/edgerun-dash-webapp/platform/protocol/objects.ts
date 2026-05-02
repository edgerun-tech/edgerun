/**
 * Object protocol helpers.
 * Handles ObjectRef resolution, object fetching, and object metadata.
 */

import { protocolClient } from "./client"
import { encodeBase64, decodeBase64, sha256 } from "./codec"
import type { ObjectRef } from "./refs"

export interface StoredObject {
  objectRef: ObjectRef
  objectId: string
  objectType: string
  sizeBytes: number
  storedAt: string
  contentHash: string
  metadata: Record<string, string>
}

export interface ObjectMetadata {
  objectId: string
  objectType: string
  sizeBytes: number
  contentHash: string
  storedAt: string
  ownerNodeId?: string
  isWasm: boolean
  isPublic: boolean
}

export async function fetchObject(objectRef: ObjectRef): Promise<Uint8Array> {
  const path = `/protocol/object/${objectRef.objectId}`
  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) {
    throw new Error(`Failed to fetch object: ${response.status}`)
  }
  return response.body
}

export async function fetchObjectMetadata(
  objectRef: ObjectRef,
): Promise<ObjectMetadata> {
  const path = `/protocol/object/${objectRef.objectId}/metadata`
  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) {
    throw new Error(`Failed to fetch metadata: ${response.status}`)
  }
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as ObjectMetadata
}

export async function storeObject(
  bytes: Uint8Array,
  objectType: string,
): Promise<ObjectRef> {
  const hash = await sha256(bytes)
  const contentHash = Array.from(hash)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("")

  const response = await protocolClient.send({
    method: "PUT",
    path: "/protocol/object",
    body: bytes,
    headers: {
      "X-Object-Type": objectType,
      "X-Content-Hash": contentHash,
    },
  })

  if (response.status !== 200 && response.status !== 201) {
    throw new Error(`Failed to store object: ${response.status}`)
  }

  const text = new TextDecoder().decode(response.body)
  const result = JSON.parse(text)
  return { kind: "object", objectId: result.objectId }
}

export function isWasmObject(metadata: ObjectMetadata): boolean {
  return metadata.isWasm || metadata.objectType === "wasm_module"
}

export function isPublicObject(metadata: ObjectMetadata): boolean {
  return metadata.isPublic
}

export async function listObjects(
  nodeId?: string,
): Promise<ObjectMetadata[]> {
  const path = nodeId ? `/protocol/objects?node=${nodeId}` : "/protocol/objects"
  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) {
    return []
  }
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as ObjectMetadata[]
}
