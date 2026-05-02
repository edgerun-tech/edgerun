/**
 * Hook for accessing capability state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  capabilityStore,
  availableCapabilities,
  capabilityCount,
  listAvailableCapabilitiesByNode,
  listCapabilitiesByNode,
  listGrantsForAppFromStore,
  listGrantsForSession,
  canSatisfy as storeCanSatisfy,
  explainMissingCapabilities as storeExplainMissing,
  loadCapabilities,
  loadGrantsForApp,
  getCapabilitySatisfaction,
} from "@/platform/state/capability-store"
import {
  capabilityRegistry,
  allDescriptors,
  getCapabilityById,
  listGrantsForApp as registryListGrantsForApp,
  listGrantsForSession as registryListGrantsForSession,
  canSatisfy as registryCanSatisfy,
  explainMissingCapabilities as registryExplainMissing,
  resolveCapabilityForAction,
  resolveCapabilityForPipeline,
} from "@/platform/registries/capability-registry"

export function useCapabilities() {
  const store = useStore(capabilityStore)
  const capabilities = useStore(availableCapabilities)
  const count = useStore(capabilityCount)
  const descriptors = useStore(allDescriptors)

  return {
    capabilities,
    descriptors,
    count,
    isLoading: store.isLoading,
    error: store.error,
    listAvailableCapabilitiesByNode,
    listCapabilitiesByNode,
    listGrantsForApp: listGrantsForAppFromStore,
    listGrantsForSession,
    canSatisfy: storeCanSatisfy,
    explainMissing: storeExplainMissing,
    loadCapabilities,
    loadGrantsForApp,
    getCapabilitySatisfaction,
    registry: {
      getCapabilityById,
      listGrantsForApp: registryListGrantsForApp,
      listGrantsForSession: registryListGrantsForSession,
      canSatisfy: registryCanSatisfy,
      explainMissing: registryExplainMissing,
      resolveForAction: resolveCapabilityForAction,
      resolveForPipeline: resolveCapabilityForPipeline,
    },
  }
}
