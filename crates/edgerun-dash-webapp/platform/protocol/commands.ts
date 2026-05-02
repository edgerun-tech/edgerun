/**
 * Command builder and helpers for protocol commands.
 * Uses generated protobuf types for CommandEnvelope and related messages.
 */

import { protocolClient } from "./client"
import { encodeBase64, decodeBase64 } from "./codec"
import type { ObjectRef } from "./refs"

export interface CommandEnvelope {
  commandId: string
  commandType: string
  targetNodeId: string
  issuerIdentity: string
  issuedAt: string
  payloadBytes: Uint8Array
  signatureBytes: Uint8Array
}

export interface CommandResult {
  commandId: string
  status: "committed" | "rejected" | "pending"
  eventRef?: ObjectRef
  error?: string
}

export interface CommandBuilder {
  commandType: string
  targetNodeId: string
  issuerIdentity: string
  payload: unknown
}

export function buildCommandEnvelope(builder: CommandBuilder): CommandEnvelope {
  const commandId = crypto.randomUUID()
  const issuedAt = new Date().toISOString()

  return {
    commandId,
    commandType: builder.commandType,
    targetNodeId: builder.targetNodeId,
    issuerIdentity: builder.issuerIdentity,
    issuedAt,
    payloadBytes: new Uint8Array(),
    signatureBytes: new Uint8Array(),
  }
}

export async function signCommand(
  envelope: CommandEnvelope,
  signFn: (bytes: Uint8Array) => Promise<Uint8Array>,
): Promise<CommandEnvelope> {
  const toSign = new Uint8Array([
    ...new TextEncoder().encode(envelope.commandId),
    ...new TextEncoder().encode(envelope.commandType),
    ...new TextEncoder().encode(envelope.targetNodeId),
    ...envelope.payloadBytes,
  ])
  const signature = await signFn(toSign)
  return { ...envelope, signatureBytes: signature }
}

export async function sendCommand(
  envelope: CommandEnvelope,
): Promise<CommandResult> {
  const request = {
    method: "POST",
    path: "/protocol/command",
    body: encodeCommandEnvelope(envelope),
  }

  try {
    const response = await protocolClient.send(request)
    if (response.status === 200) {
      return decodeCommandResult(response.body)
    }
    return {
      commandId: envelope.commandId,
      status: "rejected",
      error: `Status ${response.status}`,
    }
  } catch (err) {
    return {
      commandId: envelope.commandId,
      status: "rejected",
      error: err instanceof Error ? err.message : "Unknown error",
    }
  }
}

export function encodeCommandEnvelope(envelope: CommandEnvelope): Uint8Array {
  const parts: Uint8Array[] = [
    new TextEncoder().encode(envelope.commandId),
    new TextEncoder().encode(envelope.commandType),
    new TextEncoder().encode(envelope.targetNodeId),
    new TextEncoder().encode(envelope.issuerIdentity),
    new TextEncoder().encode(envelope.issuedAt),
    envelope.payloadBytes,
    envelope.signatureBytes,
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

export function decodeCommandResult(bytes: Uint8Array): CommandResult {
  try {
    const text = new TextDecoder().decode(bytes)
    const parsed = JSON.parse(text)
    return {
      commandId: parsed.commandId || "",
      status: parsed.status || "pending",
      eventRef: parsed.eventRef,
      error: parsed.error,
    }
  } catch {
    return { commandId: "", status: "pending" }
  }
}

export function parseCommandType(commandType: string): {
  domain: string
  action: string
} {
  const parts = commandType.split(".")
  if (parts.length >= 2) {
    return { domain: parts[0], action: parts.slice(1).join(".") }
  }
  return { domain: "unknown", action: commandType }
}
