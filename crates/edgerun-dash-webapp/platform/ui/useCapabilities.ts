/**
 * Hook for accessing capability state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  capabilityStore,
  availableCapabilities,
  capabilityCount,
  listCapabilitiesByNode,
  listGrantsForAppFromStore,
  loadCapabilities,
  loadGrantsForApp,
} from "@/platform/state/capability-store"
import { capabilityRegistry, canSatisfy, resolveCapabilityForAction } from "@/platform/registries/capability-registry"

export function useCapabilities() {
  const store = useStore(capabilityStore)
  const capabilities = useStore(availableCapabilities)
  const count = useStore(capabilityCount)

  return {
    capabilities,
    count,
    isLoading: store.isLoading,
    error: store.error,
    listCapabilitiesByNode,
    listGrantsForApp: listGrantsForAppFromStore,
    loadCapabilities,
    loadGrantsForApp,
    registry: {
      canSatisfy,
      resolveCapabilityForAction,
    },
  }
}
