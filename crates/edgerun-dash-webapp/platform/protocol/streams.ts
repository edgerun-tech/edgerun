/**
 * Stream protocol helpers.
 * Handles stream events, stream verification, and stream-related protocol messages.
 */

import { protocolClient } from "./client"
import { sha256, encodeBase64 } from "./codec"
import type { ObjectRef, EventRef } from "./refs"

export interface StreamEvent {
  eventId: string
  streamId: string
  seq: number
  eventType: string
  previousEventHash: string
  payloadBytes: Uint8Array
  signatureBytes: Uint8Array
  issuedAt: string
  issuedBy: string
}

export interface StreamRef {
  streamId: string
  nodeId: string
  writerIdentity: string
}

export interface StreamHead {
  streamId: string
  lastSeq: number
  lastEventHash: string
  lastEventId: string
  updatedAt: string
}

export interface AppendResult {
  success: boolean
  eventRef?: EventRef
  error?: string
}

export async function fetchStreamEvents(
  streamId: string,
  fromSeq?: number,
  limit?: number,
): Promise<StreamEvent[]> {
  let path = `/protocol/stream/${streamId}/events`
  const params = new URLSearchParams()
  if (fromSeq !== undefined) params.set("from", fromSeq.toString())
  if (limit !== undefined) params.set("limit", limit.toString())
  const query = params.toString()
  if (query) path += `?${query}`

  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) return []
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as StreamEvent[]
}

export async function fetchStreamHead(
  streamId: string,
): Promise<StreamHead | null> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/stream/${streamId}/head`,
  })
  if (response.status !== 200) return null
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as StreamHead
}

export async function verifyStreamContiguity(
  events: StreamEvent[],
): Promise<boolean> {
  if (events.length === 0) return true
  for (let i = 1; i < events.length; i++) {
    const prev = events[i - 1]
    const curr = events[i]
    if (curr.seq !== prev.seq + 1) return false
    const prevHash = await sha256(
      new TextEncoder().encode(prev.eventId + prev.payloadBytes),
    )
    const prevHashHex = encodeBase64(new Uint8Array(prevHash))
    if (curr.previousEventHash !== prevHashHex) return false
  }
  return true
}

export async function appendStreamEvent(
  streamId: string,
  event: Omit<StreamEvent, "eventId" | "seq" | "previousEventHash">,
): Promise<AppendResult> {
  const head = await fetchStreamHead(streamId)
  const seq = head ? head.lastSeq + 1 : 0
  const previousEventHash = head ? head.lastEventHash : ""

  const newEvent: StreamEvent = {
    ...event,
    eventId: crypto.randomUUID(),
    seq,
    previousEventHash,
  }

  const response = await protocolClient.send({
    method: "POST",
    path: `/protocol/stream/${streamId}/append`,
    body: encodeStreamEvent(newEvent),
  })

  if (response.status === 200) {
    return {
      success: true,
      eventRef: {
        kind: "event",
        eventId: newEvent.eventId,
        streamId,
        seq,
      },
    }
  }

  return {
    success: false,
    error: `Append failed: ${response.status}`,
  }
}

export function encodeStreamEvent(event: StreamEvent): Uint8Array {
  const parts: Uint8Array[] = [
    new TextEncoder().encode(event.eventId),
    new TextEncoder().encode(event.streamId),
    new TextEncoder().encode(event.seq.toString()),
    new TextEncoder().encode(event.eventType),
    new TextEncoder().encode(event.previousEventHash),
    event.payloadBytes,
    event.signatureBytes,
    new TextEncoder().encode(event.issuedAt),
    new TextEncoder().encode(event.issuedBy),
  ]
  const totalLength = parts.reduce((sum, p) => sum + p.length, 0)
  const result = new Uint8Array(totalLength)
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.length
  }
  return result
}

export async function computeEventHash(event: StreamEvent): Promise<string> {
  const bytes = new TextEncoder().encode(
    event.eventId + event.streamId + event.seq + event.eventType,
  )
  const hash = await sha256(bytes)
  return encodeBase64(new Uint8Array(hash))
}
