/**
 * Protocol references - parse/format ObjectRef, EventRef, CommandRef, NodeRef, IdentityRef.
 * No component should manually format protocol references.
 */

export interface ObjectRef {
  kind: "object"
  objectId: string
  nodeId?: string
}

export interface EventRef {
  kind: "event"
  eventId: string
  streamId?: string
  seq?: number
}

export interface CommandRef {
  kind: "command"
  commandId: string
  envelopeHash?: string
}

export interface NodeRef {
  kind: "node"
  nodeId: string
  target?: string
}

export interface IdentityRef {
  kind: "identity"
  identityId: string
  fingerprint?: string
}

export type ProtocolRef = ObjectRef | EventRef | CommandRef | NodeRef | IdentityRef

export function formatObjectRef(ref: ObjectRef): string {
  if (ref.nodeId) {
    return `object:${ref.nodeId}/${ref.objectId}`
  }
  return `object:${ref.objectId}`
}

export function formatEventRef(ref: EventRef): string {
  let s = `event:${ref.eventId}`
  if (ref.streamId) s += `@${ref.streamId}`
  if (ref.seq !== undefined) s += `#${ref.seq}`
  return s
}

export function formatCommandRef(ref: CommandRef): string {
  let s = `command:${ref.commandId}`
  if (ref.envelopeHash) s += `:${ref.envelopeHash}`
  return s
}

export function formatNodeRef(ref: NodeRef): string {
  let s = `node:${ref.nodeId}`
  if (ref.target) s += `@${ref.target}`
  return s
}

export function formatIdentityRef(ref: IdentityRef): string {
  let s = `identity:${ref.identityId}`
  if (ref.fingerprint) s += `:${ref.fingerprint}`
  return s
}

export function parseObjectRef(s: string): ObjectRef | null {
  const m = s.match(/^object:(?:(.+?)\/)?(.+)$/)
  if (!m) return null
  return { kind: "object", objectId: m[2], nodeId: m[1] || undefined }
}

export function parseEventRef(s: string): EventRef | null {
  const m = s.match(/^event:(.+?)(?:@(.+?))?(?:#(\d+))?$/)
  if (!m) return null
  return {
    kind: "event",
    eventId: m[1],
    streamId: m[2] || undefined,
    seq: m[3] ? parseInt(m[3]) : undefined,
  }
}

export function parseCommandRef(s: string): CommandRef | null {
  const m = s.match(/^command:(.+?)(?::(.+))?$/)
  if (!m) return null
  return { kind: "command", commandId: m[1], envelopeHash: m[2] || undefined }
}

export function parseNodeRef(s: string): NodeRef | null {
  const m = s.match(/^node:(.+?)(?:@(.+))?$/)
  if (!m) return null
  return { kind: "node", nodeId: m[1], target: m[2] || undefined }
}

export function parseIdentityRef(s: string): IdentityRef | null {
  const m = s.match(/^identity:(.+?)(?::(.+))?$/)
  if (!m) return null
  return { kind: "identity", identityId: m[1], fingerprint: m[2] || undefined }
}

export function parseProtocolRef(s: string): ProtocolRef | null {
  if (s.startsWith("object:")) return parseObjectRef(s)
  if (s.startsWith("event:")) return parseEventRef(s)
  if (s.startsWith("command:")) return parseCommandRef(s)
  if (s.startsWith("node:")) return parseNodeRef(s)
  if (s.startsWith("identity:")) return parseIdentityRef(s)
  return null
}

export function formatProtocolRef(ref: ProtocolRef): string {
  switch (ref.kind) {
    case "object": return formatObjectRef(ref)
    case "event": return formatEventRef(ref)
    case "command": return formatCommandRef(ref)
    case "node": return formatNodeRef(ref)
    case "identity": return formatIdentityRef(ref)
  }
}
