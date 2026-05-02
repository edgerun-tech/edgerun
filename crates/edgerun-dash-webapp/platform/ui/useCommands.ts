/**
 * Hook for accessing command state.
 */

import { useStore } from "@nanostores/react"
import {
  commandStore,
  pendingCommandCount,
  addPendingCommand,
  updateCommandResult,
  getCommandResult,
  isCommandPending,
} from "@/platform/state/command-store"
import type { CommandEnvelope, CommandResult } from "@/platform/protocol/commands"

export function useCommands() {
  const store = useStore(commandStore)
  const pendingCount = useStore(pendingCommandCount)

  return {
    pendingCommands: store.pendingCommands,
    commandHistory: store.commandHistory,
    pendingCount,
    isLoading: store.isLoading,
    error: store.error,
    addPending: addPendingCommand,
    updateResult: updateCommandResult,
    getResult: getCommandResult,
    isPending: isCommandPending,
  }
}
