/**
 * Single source of truth for command state and history.
 */

import { atom, computed } from "nanostores"
import type { CommandEnvelope, CommandResult } from "@/platform/protocol/commands"

export interface CommandStoreState {
  pendingCommands: Map<string, CommandEnvelope>
  commandHistory: Map<string, CommandResult>
  isLoading: boolean
  error: string | null
}

const initialState: CommandStoreState = {
  pendingCommands: new Map(),
  commandHistory: new Map(),
  isLoading: false,
  error: null,
}

export const commandStore = atom<CommandStoreState>(initialState)

export const pendingCommandCount = computed(commandStore, (s) =>
  s.pendingCommands.size,
)

export function addPendingCommand(command: CommandEnvelope): void {
  const state = commandStore.get()
  const newPending = new Map(state.pendingCommands)
  newPending.set(command.commandId, command)
  commandStore.set({ ...state, pendingCommands: newPending })
}

export function updateCommandResult(result: CommandResult): void {
  const state = commandStore.get()
  const newPending = new Map(state.pendingCommands)
  newPending.delete(result.commandId)
  const newHistory = new Map(state.commandHistory)
  newHistory.set(result.commandId, result)
  commandStore.set({
    ...state,
    pendingCommands: newPending,
    commandHistory: newHistory,
  })
}

export function getCommandResult(
  commandId: string,
): CommandResult | undefined {
  return commandStore.get().commandHistory.get(commandId)
}

export function isCommandPending(commandId: string): boolean {
  return commandStore.get().pendingCommands.has(commandId)
}
