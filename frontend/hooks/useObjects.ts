/**
 * Hook for accessing object state.
 */

import { useStore } from "@nanostores/react"
import {
  objectStore,
  storedObjects,
  fetchObjectMetadata,
} from "@/stores/object-store"

export function useObjects() {
  const store = useStore(objectStore)
  const objects = useStore(storedObjects)

  return {
    objects,
    isLoading: store.isLoading,
    error: store.error,
    fetchMetadata: fetchObjectMetadata,
  }
}
