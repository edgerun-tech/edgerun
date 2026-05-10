/**
 * Hook for accessing command state.
 */

import { useStore } from "@nanostores/react"
import {
  commandStore,
  pendingCommands,
  addPendingCommand,
} from "@/stores/command-store"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"

export function useCommands() {
  const store = useStore(commandStore)
  const pending = useStore(pendingCommands)

  return {
    pending,
    isLoading: store.isLoading,
    error: store.error,
    addPending: addPendingCommand,
  }
}
