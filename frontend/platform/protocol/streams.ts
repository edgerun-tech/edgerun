/**
 * Stream protocol helpers.
 * Uses generated protobuf types for stream messages.
 */

import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun } from "@/gen/edgerun/v0/common"
import { protocolClient } from "./client"

export type EventEnvelope = edgerunStream.v0.stream.EventEnvelope
export type CommandType = edgerunStream.v0.stream.CommandType

export async function fetchStreamEvents(
  streamId: string,
  fromSeq?: number,
  limit?: number,
): Promise<EventEnvelope[]> {
  let path = `/protocol/stream/${streamId}/events`
  const params = new URLSearchParams()
  if (fromSeq !== undefined) params.set("from", fromSeq.toString())
  if (limit !== undefined) params.set("limit", limit.toString())
  const query = params.toString()
  if (query) path += `?${query}`

  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) return []
  const items = JSON.parse(new TextDecoder().decode(response.body)) as Array<any>
  return items.map((obj) => edgerunStream.v0.stream.EventEnvelope.fromObject(obj))
}
