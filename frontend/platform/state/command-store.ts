/**
 * Command store for tracking pending and completed commands.
 * Uses generated protobuf types.
 */

import { atom, computed } from "nanostores"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun } from "@/gen/edgerun/v0/common"

export interface PendingCommand {
  envelope: edgerunStream.v0.stream.CommandEnvelope
  submittedAt: string
  status: "pending" | "committed" | "rejected"
}

export interface CommandStoreState {
  pending: Map<string, PendingCommand>
  isLoading: boolean
  error: string | null
}

const initialState: CommandStoreState = {
  pending: new Map(),
  isLoading: false,
  error: null,
}

export const commandStore = atom<CommandStoreState>(initialState)

export const pendingCommands = computed(commandStore, (s) =>
  Array.from(s.pending.values()),
)

export function addPendingCommand(
  envelope: edgerunStream.v0.stream.CommandEnvelope,
): void {
  const state = commandStore.get()
  const commandId = Buffer.from(envelope.command_id).toString("hex")
  const newPending = new Map(state.pending)
  newPending.set(commandId, {
    envelope,
    submittedAt: new Date().toISOString(),
    status: "pending",
  })
  commandStore.set({ ...state, pending: newPending })
}
