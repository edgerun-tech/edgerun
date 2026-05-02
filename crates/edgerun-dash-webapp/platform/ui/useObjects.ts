/**
 * Hook for accessing object state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  objectStore,
  allObjects,
  getObjectMetadata,
  getCachedContent,
  fetchObjectMetadata,
  fetchObjectContent,
  invalidateObjectCache,
} from "@/platform/state/object-store"

export function useObjects() {
  const store = useStore(objectStore)
  const objects = useStore(allObjects)

  return {
    objects,
    isLoading: store.isLoading,
    error: store.error,
    getMetadata: getObjectMetadata,
    getCachedContent,
    fetchMetadata: fetchObjectMetadata,
    fetchContent: fetchObjectContent,
    invalidateCache: invalidateObjectCache,
  }
}
