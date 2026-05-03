/**
 * Command builder and helpers for protocol commands.
 * Uses generated protobuf types for CommandEnvelope and related messages.
 */

import { edgerun } from "@/gen/edgerun/v0/common"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"

export type CommandEnvelope = edgerunStream.v0.stream.CommandEnvelope
export type CommandType = edgerunStream.v0.stream.CommandType

export function buildCommandEnvelope(
  commandType: CommandType,
  targetNode: edgerun.v0.common.NodeRef,
  issuer: edgerun.v0.common.IdentityRef,
  payload?: Uint8Array,
  delegationChain: any[] = [],
): edgerunStream.v0.stream.CommandEnvelope {
  return new edgerunStream.v0.stream.CommandEnvelope({
    envelope_version: 1,
    command_id: crypto.getRandomValues(new Uint8Array(32)),
    target_node: targetNode,
    issuer: issuer,
    command_type: commandType,
    command_version: 1,
    inline_payload: payload,
    delegation_chain: delegationChain,
  })
}

export function serializeCommandEnvelope(envelope: edgerunStream.v0.stream.CommandEnvelope): Uint8Array {
  return envelope.serialize()
}

export function deserializeCommandEnvelope(bytes: Uint8Array): edgerunStream.v0.stream.CommandEnvelope {
  return edgerunStream.v0.stream.CommandEnvelope.deserialize(bytes)
}
