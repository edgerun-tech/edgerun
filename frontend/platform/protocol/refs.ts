/**
 * Protocol reference helpers - display formatting for protocol refs.
 * Uses generated protobuf types.
 */

import { edgerun } from "@/gen/edgerun/v0/common"
import { bytesToHex } from "@/platform/utils/bytes"

export type ObjectRef = edgerun.v0.common.ObjectRef
export type EventRef = edgerun.v0.common.EventRef
export type CommandRef = edgerun.v0.common.CommandRef
export type NodeRef = edgerun.v0.common.NodeRef
export type IdentityRef = edgerun.v0.common.IdentityRef
export type ProtocolRef = ObjectRef | EventRef | CommandRef | NodeRef | IdentityRef

export function formatObjectRef(ref: ObjectRef): string {
  return `object:${bytesToHex(ref.object_id)}`
}

export function formatEventRef(ref: EventRef): string {
  return `event:${ref.seq}`
}

export function formatCommandRef(ref: CommandRef): string {
  return `command:${bytesToHex(ref.command_id)}`
}

export function formatNodeRef(ref: NodeRef): string {
  return `node:${bytesToHex(ref.node_id)}`
}

export function formatIdentityRef(ref: IdentityRef): string {
  return `identity:${bytesToHex(ref.identity_id)}`
}

export function formatProtocolRef(ref: ProtocolRef): string {
  if (ref instanceof edgerun.v0.common.ObjectRef) return formatObjectRef(ref)
  if (ref instanceof edgerun.v0.common.EventRef) return formatEventRef(ref)
  if (ref instanceof edgerun.v0.common.CommandRef) return formatCommandRef(ref)
  if (ref instanceof edgerun.v0.common.NodeRef) return formatNodeRef(ref)
  if (ref instanceof edgerun.v0.common.IdentityRef) return formatIdentityRef(ref)
  return ""
}
