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
} from "@/stores/capability-store"
import { capabilityRegistry, canSatisfy } from "@/platform/registries/capability-registry"
import { resolveCapabilityForAction, resolveCapabilityForPipeline } from "@/platform/runtime/capability-resolver"

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
      resolveCapabilityForPipeline,
    },
  }
}
